use nitro_shared::{
	output::{MessageContents, NitroOutput},
	util::DeserListOrSingle,
};
use serde::{Deserialize, Serialize};

/// Signature for a possible problem
#[derive(Serialize, Deserialize)]
pub struct Signature {
	/// A list of matchers, with each matcher being a list of segments to match from the log output.
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
}

impl Signature {
	pub fn matches(&self, log_file: &str) -> bool {
		self.matchers
			.iter()
			.any(|x| matcher_matches(x.iter(), log_file))
	}
}

fn matcher_matches<'a>(matcher: impl Iterator<Item = &'a String>, log_file: &str) -> bool {
	let mut remaining = log_file;

	for segment in matcher {
		match remaining.find(segment) {
			Some(index) => {
				let next_start = index + segment.len();
				remaining = &remaining[next_start..];
			}
			None => return false,
		}
	}

	true
}

/// Deduced problem from a signature
#[derive(Serialize, Deserialize)]
pub struct Diagnosis {
	pub id: String,
	pub ty: DiagnosisType,
	#[serde(flatten)]
	pub signature: Signature,
	pub reasons: DeserListOrSingle<String>,
	pub fixes: DeserListOrSingle<String>,
}

impl Diagnosis {
	pub fn output(&self, o: &mut impl NitroOutput) {
		o.display(MessageContents::Header("Diagnosis:".into()));
		let mut section = o.get_section();

		let severity = match &self.ty {
			DiagnosisType::Issue => "Minor Issue",
			DiagnosisType::Crashable => "Crash-Worthy",
		};

		section.display(MessageContents::property(
			"Severity",
			MessageContents::Simple(severity.into()),
		));

		if self.reasons.len() == 1 {
			section.display(MessageContents::property(
				"Possible Cause",
				MessageContents::Simple(self.reasons.first().unwrap().clone()),
			));
		} else {
			section.display(MessageContents::Header("Possible Causes".into()));
			let mut section = section.get_section();

			for reason in self.reasons.iter() {
				section.display(MessageContents::ListItem(Box::new(
					MessageContents::Simple(reason.clone()),
				)));
			}
		}

		if self.fixes.len() == 1 {
			section.display(MessageContents::property(
				"Fix",
				MessageContents::Simple(self.fixes.first().unwrap().clone()),
			));
		} else {
			section.display(MessageContents::Header("Fixes".into()));
			let mut section = section.get_section();

			for fix in self.fixes.iter() {
				section.display(MessageContents::ListItem(Box::new(
					MessageContents::Simple(fix.clone()),
				)));
			}
		}
	}
}

/// Severity of a diagnosis
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisType {
	Issue,
	/// The problem could lead to a crash
	Crashable,
}
