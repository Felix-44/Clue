use std::{ops::Range, sync::OnceLock};
use ahash::AHashMap;
use colored::*;

static mut FILES: OnceLock<AHashMap<String, String>> = OnceLock::new();

pub fn add_source_file(filename: &str, code: impl Into<String>) {
	unsafe {
		let files = match FILES.get_mut() {
			Some(files) => files,
			None => {
				if let Err(other_files) = FILES.set(AHashMap::new()) {
					let files = FILES.get_mut().unwrap();
					files.extend(other_files.into_iter());
					files
				} else {
					FILES.get_mut().unwrap()
				}
			}
		};
		files.insert(filename.into(), code.into());
	}
}

#[macro_export]
macro_rules! impl_errormessaging {
	($struct:ty) => {
		impl ErrorMessaging for $struct {
			fn get_filename(&self) -> &str {
				self.filename
			}
		
			fn is_first(&mut self, error: bool) -> bool {
				if error {
					self.errors += 1;
				}
				self.errors == 1
			}
		}
	};
}

fn get_errored_edges<'a, T: Iterator<Item = &'a str>>(
    code: &'a str,
    splitter: impl FnOnce(&'a str, char) -> T,
) -> &str {
    splitter(code, '\n')
        .next()
        .unwrap_or_default()
}

pub fn finish<T>(errors: u8, to_return: T) -> Result<T, String> {
	match errors {
		0 => {
			Ok(to_return)
		},
		1 => {
			Err(String::from("Cannot continue compiling due to the previous error!"))
		}
		n => {
			Err(format!("Cannot continue compiling due to {} previous errors!", n))
		}
	}
}

pub trait ErrorMessaging {
	fn send(
		&mut self,
		kind: ColoredString,
		message: impl Into<String>,
		line: usize,
		column: usize,
		range: Range<usize>,
		is_first: bool,
		help: Option<&str>
	) {
		let filename = self.get_filename();
		let header = format!(
			"{}{} in {}:{}:{}!",
			if is_first {
				""
			} else {
				"\n----------------------------------\n\n"
			},
			kind,
			filename,
			line,
			column
		);
		let full_message = format!(
			"{}: {}{}",
			kind,
			message.into().replace('\n', "<new line>").replace('\t', "<tab>"),
			if let Some(help) = help {
				format!("\n{}: {}", "Help".cyan().bold(), help)
			} else {
				String::from("")
			}
		);
		if let Some(code) = unsafe { FILES.get_or_init(|| AHashMap::new()).get(filename) } {
			let before_err = get_errored_edges(&code[..range.start], str::rsplit);
			let after_err = get_errored_edges(&code[range.end..], str::split);
			let errored = &code[range];
			eprintln!(
				"{}\n\n{}{}{}\n\n{}",
				header,
				before_err.trim_start(),
				errored.red().underline(),
				after_err.trim_end(),
				full_message
			)
		} else {
			eprintln!("{}\n{}", header, full_message)
		}
	}

	fn error(
		&mut self,
		message: impl Into<String>,
		line: usize,
		column: usize,
		range: Range<usize>,
		help: Option<&str>
	) {
		let is_first = self.is_first(true);
		self.send("Error".red().bold(), message, line, column, range, is_first, help)
	}

	fn expected(
		&mut self,
		expected: &str,
		got: &str,
		line: usize,
		column: usize,
		range: Range<usize>,
		help: Option<&str>)
	{
		self.error(format!("Expected '{expected}', got '{got}'"), line, column, range, help)
	}

	fn expected_before(
		&mut self,
		expected: &str,
		before: &str,
		line: usize,
		column: usize,
		range: Range<usize>,
		help: Option<&str>
	) {
		self.error(format!("Expected '{expected}' before '{before}'"), line, column, range, help)
	}

	fn warning(
		&mut self,
		message: impl Into<String>,
		line: usize,
		column: usize,
		range: Range<usize>,
		help: Option<&str>
	) {
		let is_first = self.is_first(false);
		self.send("Warning".yellow().bold(), message, line, column, range, is_first, help)
	}

	fn get_filename(&self) -> &str;

	fn is_first(&mut self, error: bool) -> bool;
}