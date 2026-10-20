use review_queue::details::description_lines;

fn lines(body: &str) -> Vec<String> {
    description_lines(body, 40, 20)
}

#[test]
fn html_comments_are_removed_even_across_lines() {
    let body = "Fixes the cache.\n<!-- Describe your change.\n     Link the issue. -->\nCloses #12. <!-- inline -->";

    assert_eq!(lines(body), vec!["Fixes the cache.", "Closes #12."]);
}

#[test]
fn images_are_removed() {
    let body = "Before:\n![screenshot](https://example.com/a.png)\nAfter: <img src=\"b.png\" width=\"300\"> done";

    assert_eq!(lines(body), vec!["Before:", "After: done"]);
}

#[test]
fn runs_of_blank_lines_become_one_and_blank_lines_at_the_ends_are_removed() {
    let body = "\n\n  \nFirst.\n\n\n\nSecond.   \n\n";

    assert_eq!(lines(body), vec!["First.", "", "Second."]);
}

#[test]
fn long_lines_wrap_at_word_boundaries_and_long_words_are_split() {
    let body = "The quick brown fox jumps over the lazy dog again.\nhttps://example.com/a/very/long/path/that/does/not/fit";

    assert_eq!(
        description_lines(body, 20, 20),
        vec![
            "The quick brown fox",
            "jumps over the lazy",
            "dog again.",
            "https://example.com/",
            "a/very/long/path/tha",
            "t/does/not/fit",
        ]
    );
}

#[test]
fn text_that_does_not_fit_is_cut_and_the_last_line_ends_with_an_ellipsis() {
    let body = "One.\nTwo.\nThree.\nFour.";

    assert_eq!(
        description_lines(body, 40, 3),
        vec!["One.", "Two.", "Three.…"]
    );
}

#[test]
fn an_empty_description_has_no_lines() {
    assert!(lines("").is_empty());
    assert!(lines("<!-- only a template -->\n\n").is_empty());
}
