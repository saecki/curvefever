use proc_macro::{Ident, Literal, TokenStream, TokenTree};
use std::borrow::Cow;
use std::fmt::Write as _;

struct Enum {
    repr: Option<Ident>,
    vis: String,
    name: String,
    rename_all: Option<Case>,
    members: Vec<Member>,
}

#[derive(Clone, Copy)]
enum Case {
    /// caseExample
    Camel,
    /// CaseExample
    Pascal,
    /// case_example
    Snake,
    /// CASE_EXAMPLE
    ScreamingSnake,
    /// case-example
    Kebab,
    /// CASE-EXAMPLE
    ScreamingKebab,
}

struct Member {
    ident: Ident,
    rename: Option<Literal>,
    value: Option<Literal>,
}

fn expect_punct_like(tokens: &mut impl Iterator<Item = TokenTree>, punct: &str) {
    match tokens.next() {
        Some(TokenTree::Punct(p)) if p.to_string() == punct => (),
        _ => panic!("expected punctuation: '{punct}'"),
    }
}

fn parse_enum(input: TokenStream) -> Enum {
    let mut tokens = input.into_iter().peekable();

    let mut rename_all = None;
    let mut repr = None;
    loop {
        match tokens.peek() {
            Some(TokenTree::Punct(p)) if p.to_string() == "#" => {
                tokens.next();

                let mut attributes = match tokens.next() {
                    Some(TokenTree::Group(g)) => g.stream().into_iter(),
                    _ => panic!("expected attribute list"),
                };

                let ident = match attributes.next() {
                    Some(TokenTree::Ident(i)) => i,
                    _ => panic!("expected attribute identifier"),
                };

                if ident.to_string() != "cods" {
                    continue;
                }

                let mut attribute_args = match attributes.next() {
                    Some(TokenTree::Group(g)) => g.stream().into_iter(),
                    _ => panic!("expected attribute args"),
                };

                match attribute_args.next() {
                    Some(TokenTree::Ident(i)) => match i.to_string().as_str() {
                        "repr" => {
                            expect_punct_like(&mut attribute_args, "=");

                            match attribute_args.next() {
                                Some(TokenTree::Ident(i)) => {
                                    repr = Some(i);
                                }
                                _ => panic!("expected repr type"),
                            }
                        }
                        "rename_all" => {
                            expect_punct_like(&mut attribute_args, "=");

                            match attribute_args.next() {
                                Some(TokenTree::Literal(l)) => {
                                    rename_all = match l.to_string().trim_matches('"') {
                                        "camelCase" => Some(Case::Camel),
                                        "PascalCase" => Some(Case::Pascal),
                                        "snake_case" => Some(Case::Snake),
                                        "SCREAMING_SNAKE_CASE" => Some(Case::ScreamingSnake),
                                        "kebab-case" => Some(Case::Kebab),
                                        "SCREAMING-KEBAB-CASE" => Some(Case::ScreamingKebab),
                                        _ => panic!("unknown case"),
                                    };
                                }
                                _ => panic!("expected rename literal"),
                            }
                        }
                        a => panic!("unexpected attribute: {a}"),
                    },
                    Some(t) => panic!("unexpected token: {}", t),
                    None => (),
                }
            }
            _ => break,
        }
    }

    let mut vis = String::new();
    if let Some(TokenTree::Ident(i)) = tokens.peek() {
        let v = i.to_string();
        if v == "pub" {
            vis = v;
            tokens.next();
        }
    }

    match tokens.next() {
        Some(TokenTree::Ident(i)) if i.to_string() == "enum" => (),
        Some(t) => panic!("expected enum keyword found {t}"),
        None => panic!("expected enum keyword"),
    }

    let name = match tokens.next() {
        Some(TokenTree::Ident(i)) => i.to_string(),
        _ => panic!("expected identifier"),
    };

    let mut body = match tokens.next() {
        Some(TokenTree::Group(g)) => g.stream().into_iter().peekable(),
        _ => panic!("expected body"),
    };

    let mut members = Vec::new();
    while let Some(t) = body.peek() {
        let rename = match t {
            TokenTree::Punct(p) if p.to_string() == "#" => {
                body.next();

                let mut attributes = match body.next() {
                    Some(TokenTree::Group(g)) => g.stream().into_iter(),
                    _ => panic!("expected attribute list"),
                };

                match attributes.next() {
                    Some(TokenTree::Ident(i)) => {
                        if i.to_string() != "cods" {
                            continue;
                        }
                    }
                    _ => panic!("expected identifier"),
                }

                let mut attribute_args = match attributes.next() {
                    Some(TokenTree::Group(g)) => g.stream().into_iter(),
                    _ => panic!("expected attribute args"),
                };

                match attribute_args.next() {
                    Some(TokenTree::Ident(i)) if i.to_string() == "rename" => {
                        expect_punct_like(&mut attribute_args, "=");

                        match attribute_args.next() {
                            Some(TokenTree::Literal(l)) => Some(l),
                            _ => panic!("expected rename literal"),
                        }
                    }
                    Some(t) => panic!("unexpected token: {}", t),
                    None => None,
                }
            }
            _ => None,
        };

        let ident = match body.next() {
            Some(TokenTree::Ident(ident)) => ident,
            _ => panic!("expected enum variant name or attributes"),
        };

        let mut value = None;
        if let Some(TokenTree::Punct(p)) = body.peek() {
            if p.to_string() == "=" {
                body.next();

                match body.next() {
                    Some(TokenTree::Literal(l)) => {
                        value = Some(l);
                    }
                    _ => panic!("expected variant value after ="),
                }
            }
        }

        members.push(Member {
            ident,
            rename,
            value,
        });

        match body.next() {
            Some(TokenTree::Punct(p)) if p.to_string() == "," => (),
            _ => panic!("expected ,"),
        }
    }

    Enum {
        vis,
        name,
        rename_all,
        members,
        repr,
    }
}

fn transform_case(input: &str, case: Case) -> Cow<str> {
    match case {
        Case::Camel => {
            let mut output = input.to_string();
            output[0..1].make_ascii_lowercase();
            Cow::Owned(output)
        }
        Case::Pascal => Cow::Borrowed(input),
        Case::Snake => {
            let mut output = String::with_capacity(input.len() + 3);
            let mut iter = input.chars();
            output.push(iter.next().unwrap().to_ascii_lowercase());
            for c in iter {
                if c.is_ascii_uppercase() {
                    output.push('_');
                }
                output.push(c.to_ascii_lowercase());
            }
            Cow::Owned(output)
        }
        Case::ScreamingSnake => {
            let mut output = String::with_capacity(input.len() + 3);
            let mut iter = input.chars();
            output.push(iter.next().unwrap().to_ascii_uppercase());
            for c in iter {
                if c.is_ascii_uppercase() {
                    output.push('_');
                }
                output.push(c.to_ascii_uppercase());
            }
            Cow::Owned(output)
        }
        Case::Kebab => {
            let mut output = String::with_capacity(input.len() + 3);
            let mut iter = input.chars();
            output.push(iter.next().unwrap().to_ascii_lowercase());
            for c in iter {
                if c.is_ascii_uppercase() {
                    output.push('-');
                }
                output.push(c.to_ascii_lowercase());
            }
            Cow::Owned(output)
        }
        Case::ScreamingKebab => {
            let mut output = String::with_capacity(input.len() + 3);
            let mut iter = input.chars();
            output.push(iter.next().unwrap().to_ascii_uppercase());
            for c in iter {
                if c.is_ascii_uppercase() {
                    output.push('-');
                }
                output.push(c.to_ascii_uppercase());
            }
            Cow::Owned(output)
        }
    }
}

#[proc_macro_derive(EnumDisplay, attributes(cods))]
pub fn derive_display(input: TokenStream) -> TokenStream {
    let Enum {
        name,
        rename_all,
        members,
        ..
    } = parse_enum(input);

    let mut output = format!(
        "impl std::fmt::Display for {name} {{
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
                match self {{"
    );

    for m in members {
        let m_ident = m.ident.to_string();
        let _ = match m.rename {
            Some(r) => write!(output, "Self::{m_ident} => write!(f, {r}),"),
            None => match rename_all {
                Some(case) => {
                    let new_ident = transform_case(&m_ident, case);
                    write!(output, "Self::{m_ident} => write!(f, \"{new_ident}\"),")
                }
                None => write!(output, "Self::{m_ident} => write!(f, \"{m_ident}\"),"),
            },
        };
    }

    output.push_str("}}}");
    output.parse().unwrap()
}

#[proc_macro_derive(EnumFromStr, attributes(cods))]
pub fn derive_from_str(input: TokenStream) -> TokenStream {
    let Enum {
        name,
        rename_all,
        members,
        ..
    } = parse_enum(input);

    let mut output = format!(
        "impl std::str::FromStr for {name} {{
            type Err = ();

            fn from_str(input: &str) -> Result<Self, Self::Err> {{
                match input {{"
    );

    for m in members {
        let m_ident = m.ident.to_string();
        let _ = match m.rename {
            Some(r) => write!(output, "{r} => Ok(Self::{m_ident}),"),
            None => match rename_all {
                Some(case) => {
                    let new_ident = transform_case(&m_ident, case);
                    write!(output, "\"{new_ident}\" => Ok(Self::{m_ident}),")
                }
                None => write!(output, "\"{m_ident}\" => Ok(Self::{m_ident}),"),
            },
        };
    }

    let _ = write!(output, "_ => Err(()),");

    output.push_str("}}}");
    output.parse().unwrap()
}

#[proc_macro_derive(EnumTryFromRepr, attributes(cods))]
pub fn derive_try_from_repr(input: TokenStream) -> TokenStream {
    let Enum {
        name,
        repr,
        members,
        ..
    } = parse_enum(input);

    let repr = match repr {
        Some(s) => s.to_string(),
        None => "compile_error!(\"no cods(repr = <repr>) defined\")".into(),
    };

    let mut output = format!(
        "impl TryFrom<{repr}> for {name} {{
            type Error = ();

            fn try_from(input: {repr}) -> Result<Self, Self::Error> {{
                match input {{"
    );

    for m in members {
        let m_ident = m.ident;
        let m_value = match m.value {
            Some(s) => s.to_string(),
            None => format!("compile_error!(\"no value defined for variant {m_ident}\")"),
        };
        let _ = write!(output, "{m_value} => Ok(Self::{m_ident}),");
    }

    let _ = write!(output, "_ => Err(()),");

    output.push_str("}}}");
    output.parse().unwrap()
}

#[proc_macro_derive(EnumMembersArray, attributes(cods))]
pub fn derive_members_array(input: TokenStream) -> TokenStream {
    let Enum {
        vis, name, members, ..
    } = parse_enum(input);

    let count = members.len();
    let mut output = format!(
        "impl {name} {{
            {vis} const fn members() -> &'static [{name}; {count}] {{
                &[\n"
    );

    for m in members {
        let m_ident = m.ident.to_string();
        let _ = writeln!(output, "{name}::{m_ident},");
    }

    let _ = write!(
        output,
        "       ]
            }}
        }}"
    );

    output.parse().unwrap()
}
