use botica_astgrep::{AstGrepRule, AstGrepSet, Hit};
use std::path::Path;

const RULES: [&str; 3] = [
    include_str!("../../../rules/logging/log-no-print/python.yml"),
    include_str!("../../../rules/data-models/no-dict-literal-return/python.yml"),
    include_str!("../../../rules/data-models/no-dict-call-return/python.yml"),
];

const SNIPPETS: [&str; 6] = [
    "print('x')\n",
    "def f():\n    return {'a': 1}\n",
    "    return dict(a=1)\n",
    "if __name__ == '__main__':\n    print('ok')\n",
    "(((",
    "# ñé 日本\n",
];

fn rule_set() -> AstGrepSet {
    let rules = RULES
        .iter()
        .flat_map(|yaml| AstGrepRule::from_yaml(yaml).unwrap())
        .collect();
    AstGrepSet::new(rules)
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap() % bound.max(1)
    }
}

fn boundary(text: &str, mut index: usize) -> usize {
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn sorted(mut hits: Vec<Hit>) -> Vec<Hit> {
    hits.sort_by(|a, b| (a.start, &a.rule_id, a.end).cmp(&(b.start, &b.rule_id, b.end)));
    hits
}

#[test]
fn incremental_edits_give_the_same_hits_as_a_fresh_parse() {
    let set = rule_set();
    let path = Path::new("pkg/mod.py");
    let initial = "import os\n\n\ndef load(path):\n    print(path)\n    return {'p': path}\n";
    let mut buffer = set.open(path, initial).unwrap();
    let mut rng = Lcg(42);

    for step in 0..300 {
        let text = buffer.text().to_owned();
        let start = boundary(&text, rng.next(text.len() + 1));
        let end = boundary(&text, (start + rng.next(12)).min(text.len()));
        let snippet = if rng.next(4) == 0 {
            ""
        } else {
            SNIPPETS[rng.next(SNIPPETS.len())]
        };

        buffer.edit(start, end - start, snippet).unwrap();

        let expected = format!("{}{}{}", &text[..start], snippet, &text[end..]);
        assert_eq!(buffer.text(), expected, "text diverged at step {step}");
        assert_eq!(
            sorted(set.check_buffer(&buffer)),
            sorted(set.check(path, &expected)),
            "hits diverged at step {step}"
        );
    }
}
