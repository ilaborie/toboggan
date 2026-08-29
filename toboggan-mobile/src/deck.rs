//! Pairing the two channels a deck arrives on.

use std::sync::Arc;

use toboggan_core::{Slide as CoreSlide, TalkResponse};
use tokio::sync::watch;
use tracing::warn;

use crate::types::Slide;

/// The deck as it stands right now, each slide carrying its own step count.
///
/// Slides and step counts arrive on two independent `watch` channels, filled by
/// two separate fetches, and nothing makes them the same length at the same
/// instant. The pairing therefore happens here, once, rather than at each of the
/// three call sites that used to repeat it — and the app is handed slides that
/// already know their step count instead of a second list to index against.
///
/// A length disagreement is exactly the "not computed" case
/// [`TalkResponse::step_counts`] documents, so every slide then reports `None`
/// rather than a count belonging to some other slide.
pub(crate) fn deck_snapshot(
    slides_rx: &watch::Receiver<Arc<[CoreSlide]>>,
    talk_rx: &watch::Receiver<Option<TalkResponse>>,
) -> Vec<Slide> {
    let slides = slides_rx.borrow();
    let talk = talk_rx.borrow();

    let step_counts = match talk.as_ref().map(|talk| talk.step_counts.as_slice()) {
        Some(counts) if counts.len() == slides.len() => Some(counts),
        // Empty is the ordinary "the server has not computed them" case and is
        // not worth a line; any other length is a real disagreement.
        Some(counts) if !counts.is_empty() => {
            warn!(
                slides = slides.len(),
                step_counts = counts.len(),
                "Step counts do not line up with the slides they describe; \
                 reporting every slide as uncounted"
            );
            None
        }
        _ => None,
    };

    slides
        .iter()
        .enumerate()
        .map(|(index, slide)| {
            let step_count = step_counts.and_then(|counts| counts.get(index).copied());
            Slide::from_core_slide(slide, step_count)
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_core::Slide as CoreSlide;

    use super::*;

    /// The two receivers a snapshot reads from.
    ///
    /// The senders are dropped on purpose: a `watch::Receiver` still reads the
    /// last value it was given, and these tests never send another.
    fn channels(
        slides: Vec<CoreSlide>,
        talk: Option<TalkResponse>,
    ) -> (
        watch::Receiver<Arc<[CoreSlide]>>,
        watch::Receiver<Option<TalkResponse>>,
    ) {
        let (_slides_tx, slides_rx) = watch::channel::<Arc<[CoreSlide]>>(Arc::from(slides));
        let (_talk_tx, talk_rx) = watch::channel(talk);
        (slides_rx, talk_rx)
    }

    fn talk_with(titles: &[&str], step_counts: Vec<usize>) -> TalkResponse {
        TalkResponse {
            titles: titles.iter().map(|title| (*title).to_owned()).collect(),
            step_counts,
            ..TalkResponse::default()
        }
    }

    /// The ordinary case: the counts line up and each slide gets its own.
    #[test]
    fn a_slide_carries_the_count_that_belongs_to_it() {
        let slides = vec![CoreSlide::default(), CoreSlide::default()];
        let talk = talk_with(&["one", "two"], vec![3, 0]);
        let (slides_rx, talk_rx) = channels(slides, Some(talk));

        let counts = deck_snapshot(&slides_rx, &talk_rx)
            .iter()
            .map(|slide| slide.step_count)
            .collect::<Vec<_>>();
        assert_eq!(counts, vec![Some(3), Some(0)]);
    }

    /// Two independent channels can disagree. Padding the short one with zeros
    /// put one slide's progress against another slide's name; saying "uncounted"
    /// keeps the deck the right length and tells no lies about it.
    #[test]
    fn a_length_disagreement_makes_every_slide_uncounted() {
        let slides = vec![
            CoreSlide::default(),
            CoreSlide::default(),
            CoreSlide::default(),
        ];
        let talk = talk_with(&["one", "two"], vec![3, 1]);
        let (slides_rx, talk_rx) = channels(slides, Some(talk));

        let deck = deck_snapshot(&slides_rx, &talk_rx);
        assert_eq!(deck.len(), 3, "the deck keeps every slide it has");
        assert!(deck.iter().all(|slide| slide.step_count.is_none()));
    }

    /// Before the talk lands there is nothing to pair with, and that is not an
    /// error — just an unknown.
    #[test]
    fn slides_without_a_talk_are_uncounted() {
        let slides = vec![CoreSlide::default()];
        let (slides_rx, talk_rx) = channels(slides, None);

        let deck = deck_snapshot(&slides_rx, &talk_rx);
        assert_eq!(deck.len(), 1);
        assert_eq!(deck.first().and_then(|slide| slide.step_count), None);
    }

    #[test]
    fn an_empty_deck_is_empty() {
        let (slides_rx, talk_rx) = channels(vec![], None);
        assert!(deck_snapshot(&slides_rx, &talk_rx).is_empty());
    }
}
