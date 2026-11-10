use std::sync::mpsc;

use futures::executor::block_on;

use super::*;

#[test]
fn a_notice_becomes_one_log_line_and_an_empty_one_is_refused() {
	let (write_line, lines) = mpsc::channel();
	let notifier = LogNotifier::new(move |line| write_line.send(line.to_string()).unwrap());

	let shown = block_on(notifier.notify("saved".into()));
	let empty = block_on(notifier.notify("  ".into()));

	assert_eq!(
		(shown, empty, lines.try_iter().collect::<Vec<_>>()),
		(
			Ok(()),
			Err(NotifierError::Rejected { reason: "a notice needs text".into() }),
			vec!["[host-provider-notifier-log] notice: saved".to_string()],
		),
	);
}
