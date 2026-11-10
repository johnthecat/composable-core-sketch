//! The `UserConfirmation` extension point: the host asks the user before the
//! core goes on.

use host_macros::{extension_error, extension_point, extension_type};
use protocol::{Bytes, Bytes32};

/// What the host asks the user to confirm.
#[extension_type]
pub enum UserConfirmationReview {
	/// A product asks to sign raw bytes.
	SignRaw {
		/// The product asking.
		product_id: String,
		/// Public key of the account to sign with.
		public_key: Bytes32,
		/// The bytes to sign, as the product sent them.
		payload: Bytes,
	},
}

/// Why the host could not ask the user.
#[extension_error]
pub enum UserConfirmationError {}

/// Asks the user to confirm what a product asked for.
#[extension_point]
pub trait UserConfirmation: Send + Sync {
	/// Show `review` and resolve with whether the user confirmed it.
	async fn confirm_user_action(
		&self,
		review: UserConfirmationReview,
	) -> Result<bool, UserConfirmationError>;
}
