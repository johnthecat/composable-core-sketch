use host_extension::{Notifier, NotifierError};

/// Notifier that writes each notice as one log line.
pub struct LogNotifier {
	write_line: Box<dyn Fn(&str) + Send + Sync>,
}

impl LogNotifier {
	/// A notifier that hands each log line to `write_line`, which is where the
	/// platform's log is.
	pub fn new(write_line: impl Fn(&str) + Send + Sync + 'static) -> Self {
		Self { write_line: Box::new(write_line) }
	}
}

#[async_trait::async_trait]
impl Notifier for LogNotifier {
	async fn notify(&self, text: String) -> Result<(), NotifierError> {
		if text.trim().is_empty() {
			return Err(NotifierError::Rejected { reason: "a notice needs text".into() });
		}
		(self.write_line)(&format!("[host-provider-notifier-log] notice: {text}"));
		Ok(())
	}
}

#[cfg(test)]
mod tests;
