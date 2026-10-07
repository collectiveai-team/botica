use anyhow::Result;
use botica_astgrep::{AstGrepRule, AstGrepSet};
use botica_core::{Engine, FileSizeGuard, NativeRule, NoUtils};

const EMBEDDED: [&str; 11] = [
    include_str!("../../../../scaffolding/templates/ast-grep-rules/api-schemas-extra-forbid.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/arch-database-package.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/cli-typed-framework.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/log-get-logger.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/log-no-print.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-alias.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-call-return.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-literal-return.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/no-dict-return-annotation.yml"),
    include_str!("../../../../scaffolding/templates/ast-grep-rules/settings-module.yml"),
    include_str!("../rules/no-console-log.yml"),
];

pub fn build_engine(with_astgrep: bool) -> Result<Engine> {
    let native: Vec<Box<dyn NativeRule>> =
        vec![Box::new(FileSizeGuard::default()), Box::new(NoUtils)];
    let mut rules = Vec::new();
    if with_astgrep {
        for yaml in EMBEDDED {
            rules.extend(AstGrepRule::from_yaml(yaml)?);
        }
    }
    Ok(Engine::new(native, AstGrepSet::new(rules)))
}
