//! The `Notifier` extension point, the RFC's example: notices shown to the
//! user.

use host_macros::{extension_error, extension_point};

/// Why a notice was not shown.
#[extension_error]
#[derive(PartialEq)]
pub enum NotifierError {
	/// The platform or the host refused to show it.
	#[error("refused: {reason}")]
	Rejected {
		/// Human-readable reason.
		reason: String,
	},
}

/// Shows a notice to the user.
#[extension_point]
pub trait Notifier: Send + Sync {
	/// Shows `text` and resolves once the notice is on screen.
	async fn notify(&self, text: String) -> Result<(), NotifierError>;
}
