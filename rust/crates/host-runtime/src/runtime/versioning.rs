//! Runs a handler in latest terms and answers in the caller's version.

use futures::StreamExt;
use protocol::{
	CallError, Subscription,
	versioned::{FromLatest, IntoLatest},
};

/// A request unwrapped to its latest payload, with the version it arrived in.
pub struct VersionedRequest<Request> {
	request: Request,
	version: u8,
}

/// Unwrap `request` to its latest payload, remembering its version.
pub fn version<Request: IntoLatest>(request: Request) -> VersionedRequest<Request::Latest> {
	VersionedRequest { version: request.version(), request: request.into_latest() }
}

impl<Request> VersionedRequest<Request> {
	/// Run `handler` on the latest request, and convert what it returns,
	/// including every early return and `?` failure, to the request's version.
	pub async fn exec<Output: VersionedOutput, Fut: Future<Output = Output::Latest>>(
		self,
		handler: impl FnOnce(Request) -> Fut,
	) -> Output {
		Output::from_latest(handler(self.request).await, self.version)
	}
}

/// What a protocol method returns, convertible from its latest form.
pub trait VersionedOutput {
	/// The output in latest terms.
	type Latest;

	/// Build the output for a caller at `version`.
	fn from_latest(latest: Self::Latest, version: u8) -> Self;
}

impl<Response: FromLatest, Error: FromLatest> VersionedOutput
	for Result<Response, CallError<Error>>
{
	type Latest = Result<Response::Latest, CallError<Error::Latest>>;

	fn from_latest(latest: Self::Latest, version: u8) -> Self {
		latest
			.map(|response| Response::from_latest(response, version))
			.map_err(|error| error.map_domain(|domain| Error::from_latest(domain, version)))
	}
}

impl<Item, Error> VersionedOutput for Subscription<Item, CallError<Error>>
where
	Item: FromLatest + 'static,
	Error: FromLatest + 'static,
	Item::Latest: 'static,
	Error::Latest: 'static,
{
	type Latest = Subscription<Item::Latest, CallError<Error::Latest>>;

	fn from_latest(latest: Self::Latest, version: u8) -> Self {
		Self::new(latest.map(move |result| VersionedOutput::from_latest(result, version)))
	}
}
