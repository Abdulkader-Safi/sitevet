use crate::text::{simhash, tokenize};

fn bits(a: &str, b: &str) -> u32 {
    let (Some(x), Some(y)) = (simhash(&tokenize(a)), simhash(&tokenize(b))) else { panic!("too short") };
    x.iter().zip(&y).map(|(p, q)| (p ^ q).count_ones()).sum()
}

const BASE: &str = "Good bread starts with flour, water, salt and a little patience on a quiet morning. \
Mix the dough until it holds together, then leave it covered for twenty minutes. \
Folding the dough every half hour builds strength without hard kneading on the counter. \
A warm kitchen speeds the first rise, while a cool one gives a deeper flavour overnight. \
Shape the loaf gently so the bubbles inside survive the move into the basket. \
Dust the basket with rice flour, because wheat flour tends to stick to damp dough. \
Heat a heavy pot in the oven for forty minutes before the loaf goes in. \
Score the top with a sharp blade so steam escapes along a line you chose. \
Bake with the lid on first, then take it off to let the crust darken. \
Tap the bottom of the loaf: a hollow sound means the inside has finished baking. \
Let the bread cool on a rack for an hour, or the crumb turns gummy. \
Store the loaf cut side down on a board and it keeps for two days.";

#[test]
fn one_changed_paragraph_is_near_and_unrelated_text_is_far() {
    let edited = BASE.replace(
        "Dust the basket with rice flour, because wheat flour tends to stick to damp dough.",
        "Semolina in the basket works too, and it gives the crust a golden colour and a crunch.",
    );
    let unrelated = "Tomatoes need full sun, deep watering twice a week and a stake before the stems get heavy. \
        Pinch the side shoots that grow between the main stem and the branches. Mulch keeps the soil damp and the \
        roots cool through summer. Feed the plants with compost when the first fruit sets, then every two weeks after \
        that. Pick the fruit when it turns fully red and gives slightly under a gentle squeeze.";
    assert_eq!(bits(BASE, BASE), 0);
    let near = bits(BASE, &edited);
    let far = bits(BASE, unrelated);
    assert!(near <= super::NEAR_DUP_BITS, "one paragraph: {near} bits");
    assert!(far > 100, "unrelated: {far} bits");
    assert!(simhash(&tokenize("too few words")).is_none());
}
