use std::sync::{Arc, Mutex};

use futures::executor::block_on;

use super::*;

#[test]
fn a_notice_becomes_one_log_line_and_an_empty_one_is_refused() {
	let lines = Arc::new(Mutex::new(Vec::new()));
	let notifier = LogNotifier::new({
		let lines = lines.clone();
		move |line| lines.lock().unwrap().push(line.to_string())
	});

	let shown = block_on(notifier.notify("saved".into()));
	let empty = block_on(notifier.notify("  ".into()));

	assert_eq!(
		(shown, empty, lines.lock().unwrap().clone()),
		(
			Ok(()),
			Err(NotifierError::Rejected { reason: "a notice needs text".into() }),
			vec!["[host-provider-notifier-log] notice: saved".to_string()],
		),
	);
}
