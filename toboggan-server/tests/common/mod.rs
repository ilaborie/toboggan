#[allow(dead_code)]
use toboggan_core::{Date, Slide, Talk};

#[allow(dead_code)]
pub fn create_test_talk() -> Talk {
    Talk::new("Test Talk")
        .with_date(Date::ymd(2025, 1, 1))
        .add_slide(Slide::cover("Cover Slide"))
        .add_slide(Slide::new("Second Slide"))
        .add_slide(Slide::new("Third Slide"))
}

#[allow(dead_code)]
pub fn create_multi_slide_talk() -> Talk {
    Talk::new("Multi-Client Sync Test Talk")
        .with_date(Date::ymd(2025, 1, 25))
        .add_slide(Slide::cover("Cover Slide").with_body("This is the cover slide"))
        .add_slide(
            Slide::new("First Content Slide")
                .with_body("This is the first content slide")
                .with_notes("Notes for first slide"),
        )
        .add_slide(Slide::new("Second Content Slide").with_body("This is the second content slide"))
        .add_slide(Slide::new("Final Slide").with_body("This is the final slide"))
}
