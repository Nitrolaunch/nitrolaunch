use nitro_shared::{
	output::{MessageContents, NitroOutput},
	util::DeserListOrSingle,
};
use serde::{Deserialize, Serialize};

pub static KNOWN_MALWARE: &[(&str, &str)] = &[
	(
		"a04a5949eebb67ffe993317769dd9453accb071abfd67d4df94d8482801660ae",
		"Oringo Client",
	),
	(
		"42adbf087c2e2017944711808201bc2369280929a8c88e64efaf2722233bdf41",
		"Visomod",
	),
];

/// Signature for a possible threat
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Signature {
	/// A list of matchers, with each matcher being a list of segments to match from a file.
	///
	/// Each matcher is an AND statement, and all the matchers are combined with an OR.
	///
	/// For example:
	/// ```
	/// [
	/// 	["foo", "bar"],
	/// 	["baz"]
	/// ]
	/// ```
	/// Means (foo AND bar) OR (baz)
	///
	/// Each matcher also matches any characters in between. Think of them like a regex with a `*` between each entry.
	pub matchers: Vec<DeserListOrSingle<String>>,
	/// Matchers that only operate on the constant pool of class files
	pub constant_matchers: Vec<DeserListOrSingle<String>>,
	/// Matchers that match a whole UTF8 entry of a class file constant pool
	pub whole_constant_matchers: Vec<DeserListOrSingle<String>>,
	/// Operating systems this signature occurs on
	pub os: DeserListOrSingle<String>,
	/// Whether each occurrence of this signature should be compounded, or only one should count
	pub repeat: bool,
}

impl Signature {
	pub fn matches(&self, file: &[u8], constant_pool_end: Option<usize>) -> bool {
		if let Some(end) = constant_pool_end {
			if self
				.constant_matchers
				.iter()
				.any(|x| matcher_matches(x.iter(), &file[0..end]))
			{
				return true;
			}
		}

		self.matchers
			.iter()
			.any(|x| matcher_matches(x.iter(), file))
	}
}

fn matcher_matches<'a>(matcher: impl Iterator<Item = &'a String>, file: &[u8]) -> bool {
	let mut remaining = file;

	for segment in matcher {
		let segment = segment.as_bytes();
		if let Some(pos) = memchr::memmem::find(file, segment) {
			let next_start = pos + segment.len();
			remaining = &remaining[next_start..];
		} else {
			return false;
		}
	}

	true
}

/// Detected potential threat from a signature
#[derive(Serialize, Deserialize, Clone)]
pub struct Threat {
	pub id: String,
	pub ty: ThreatType,
	#[serde(flatten)]
	pub signature: Signature,
	pub score: u16,
}

impl Threat {
	pub fn output(&self, o: &mut impl NitroOutput, compact: bool) {
		if compact {
			o.display(MessageContents::property(
				&self.id,
				MessageContents::Simple(self.score.to_string()),
			));
			return;
		}

		o.display(MessageContents::Header(format!("Threat: {}", self.id)));
		let mut section = o.get_section();

		let severity = match &self.ty {
			ThreatType::Malware => "Known Malware",
			ThreatType::MalwarePattern => "Malware Pattern",
			ThreatType::Suspicious => "Suspicious",
			ThreatType::Smell => "Small",
		};

		section.display(MessageContents::property(
			"Severity",
			MessageContents::Simple(severity.into()),
		));

		section.display(MessageContents::property(
			"Score",
			MessageContents::Simple(self.score.to_string()),
		));
	}
}

/// Severity of a diagnosis
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ThreatType {
	/// Direct known malware class name
	Malware,
	/// Common direct pattern in malware, like accessing Discord or Chrome data
	MalwarePattern,
	/// Very peculiar, but can happen in regular code
	Suspicious,
	/// Something suspect that can still be normal, like reading from a file or sending network requests
	Smell,
}

/// Mitigation derived from the final threat score
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Mitigation {
	/// Nothing to be done, looks good
	Benign,
	/// Worth a warning
	Strange,
	/// Probably malware
	Detection,
}

impl Mitigation {
	pub fn from_score(score: u16) -> Self {
		match score {
			0..60 => Self::Benign,
			60..75 => Self::Strange,
			75.. => Self::Detection,
		}
	}
}

/// Gets the position of the end of the constant pool in a Java .class file
pub fn constant_pool_end(data: &[u8]) -> Option<usize> {
	if data.len() < 10 {
		return None;
	}

	if &data[0..4] != b"\xCA\xFE\xBA\xBE" {
		return None;
	}

	let cp_count = u16::from_be_bytes([data[8], data[9]]) as usize;

	let mut offset = 10;
	let mut i = 1;

	while i < cp_count {
		let tag = *data.get(offset)?;
		offset += 1;

		match tag {
			1 => {
				let len = u16::from_be_bytes([*data.get(offset)?, *data.get(offset + 1)?]) as usize;

				offset += 2 + len;
			}

			3 | 4 => offset += 4,

			5 | 6 => {
				offset += 8;
				i += 1; // extra slot
			}

			7 | 8 | 16 | 19 | 20 => offset += 2,

			9 | 10 | 11 | 12 | 17 | 18 => offset += 4,

			15 => offset += 3,

			_ => return None,
		}

		i += 1;
	}

	Some(offset)
}

/// Checks for known malware
pub fn check_known_malware(hash: &str) -> Option<Threat> {
	let name = KNOWN_MALWARE.iter().find(|x| x.0 == hash).map(|x| x.1)?;

	Some(Threat {
		id: format!("Known Malware: {name}"),
		ty: ThreatType::Malware,
		signature: Signature::default(),
		score: 150,
	})
}
