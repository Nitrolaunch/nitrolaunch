use nitro_shared::{
	output::{MessageContents, NitroOutput},
	util::DeserListOrSingle,
};
use serde::{Deserialize, Serialize};

/// Signature for a possible threat
#[derive(Serialize, Deserialize, Clone)]
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
	/// Operating systems this signature occurs on
	#[serde(default)]
	pub os: DeserListOrSingle<String>,
	/// Whether each occurrence of this signature should be compounded, or only one should count
	#[serde(default)]
	pub repeat: bool,
}

impl Signature {
	pub fn matches(&self, file: &[u8], os: &str) -> bool {
		if !self.os.is_empty() && !self.os.iter().any(|x| x == os) {
			return false;
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
