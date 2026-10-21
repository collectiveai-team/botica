//! `route-auth`: a route of FastAPI gets its caller from a dependency of the configuration.

use std::collections::BTreeMap;

use botica_core::registry::{self, Context, Options};
use botica_core::{Diagnostic, Document};

fn options(dependencies: &[&str], public: &[&str]) -> Options {
    let list = |names: &[&str]| {
        toml::Value::Array(
            names
                .iter()
                .map(|name| toml::Value::String((*name).to_owned()))
                .collect(),
        )
    };
    BTreeMap::from([
        ("dependencies".to_owned(), list(dependencies)),
        ("public".to_owned(), list(public)),
    ])
}

fn unguarded(options: &Options, text: &str) -> Vec<String> {
    let rule = registry::build("route-auth", options, &Context::default()).expect("a native rule");
    let mut found: Vec<Diagnostic> = Vec::new();
    rule.check(&Document::new("routes.py", text), &mut found);
    found
        .iter()
        .map(|finding| format!("{}:{}", finding.line, &text[finding.start..finding.end]))
        .collect()
}

#[test]
fn without_dependencies_the_rule_reports_nothing() {
    let text = "@app.get('/')\ndef index(): ...\n";
    assert!(unguarded(&options(&[], &[]), text).is_empty());
}

#[test]
fn a_route_with_no_guard_is_reported_at_its_decorator() {
    let text = "@router.get('/a')\n@router.head('/a')\nasync def a(): ...\n";
    assert_eq!(
        unguarded(&options(&["get_current_user"], &[]), text),
        ["1:@router.get('/a')"]
    );
}

#[test]
fn a_guard_in_a_parameter_the_decorator_or_the_router_counts() {
    let text = "\
router = APIRouter(dependencies=[Depends(get_current_user)])
other = APIRouter()

@router.get('/a')
def a(): ...

@other.get('/b', dependencies=[Security(get_current_user, scopes=['x'])])
def b(): ...

@other.get('/c')
def c(user: User = Depends(auth.get_current_user)): ...

@other.get('/d')
def d(): ...
";
    assert_eq!(
        unguarded(&options(&["get_current_user"], &[]), text),
        ["13:@other.get('/d')"]
    );
}

#[test]
fn an_alias_of_the_module_or_one_the_configuration_names_guards_a_parameter() {
    let text = "\
CurrentUser = Annotated[User, Depends(get_current_user)]

@app.get('/a')
def a(user: CurrentUser): ...

@app.get('/b')
def b(admin: Admin): ...

@app.get('/c')
def c(user: Other): ...
";
    assert_eq!(
        unguarded(&options(&["get_current_user", "Admin"], &[]), text),
        ["9:@app.get('/c')"]
    );
}

#[test]
fn a_public_route_and_a_function_that_is_no_route_are_left_alone() {
    let text = "\
@app.get('/health')
def health(): ...

@functools.cache
def settings(): ...

@app.middleware('http')
async def timing(request, call_next): ...
";
    assert!(unguarded(&options(&["get_current_user"], &["health"]), text).is_empty());
}

#[test]
fn another_dependency_does_not_guard_a_route() {
    let text = "@app.post('/a')\ndef a(db: Session = Depends(get_session)): ...\n";
    assert_eq!(
        unguarded(&options(&["get_current_user"], &[]), text),
        ["1:@app.post('/a')"]
    );
}
