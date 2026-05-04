use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{parse_macro_input, FnArg, ItemFn, Pat, Type};

/* Example Conversion:
fn f(x: &mut usize, d: usize) -> usize {
    if d < 5 {
       return 0;
    }
    *x + d
}

fn f(worker: &mut Worker, input: (&mut usize, usize)) -> usize {
    let (x, d) = input;
    if d < 5 {
        return 0;
    }
    *x + d
}
*/

#[proc_macro_attribute]
pub fn lace_task(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(input as ItemFn);
    assert!(
        func.sig.constness.is_none(),
        "const fn cannot be used as lace task"
    );
    assert!(
        func.sig.asyncness.is_none(),
        "async fn cannot be used as lace task"
    );

    // collect the function arguments
    let mut arg_ids: Vec<Pat> = Vec::with_capacity(func.sig.inputs.len());
    let mut arg_types: Vec<Type> = Vec::with_capacity(func.sig.inputs.len());
    for ipt in &func.sig.inputs {
        match ipt {
            FnArg::Receiver(_) => panic!("TODO: handle self parameter in lace task"),
            FnArg::Typed(t) => {
                arg_ids.push((*t.pat).clone());
                arg_types.push((*t.ty).clone());
            }
        }
    }
    func.sig.inputs.clear();
    // add an argument for the worker
    func.sig.inputs.push(syn::parse_quote! {
        __lace_task_worker: &mut Worker
    });
    // add the original function arguments
    if !arg_ids.is_empty() {
        func.sig.inputs.push(syn::parse_quote! {
            __lace_input: (#(#arg_types),*)
        });
        func.block.stmts.splice(
            0..0,
            [syn::parse_quote! {
                let (#(#arg_ids),*) = __lace_input;
            }],
        );
    } else {
        func.sig.inputs.push(syn::parse_quote! {
            _: ()
        });
    }

    func.block.stmts.splice(
        0..0,
        [
            syn::parse_quote! {
                #[allow(unused_macros)]
                macro_rules! call {
                    ($($task:ident)::+($($args:expr),*)) => {
                        $($task)::+(__lace_task_worker, ($($args),*))
                    }
                }
            },
            syn::parse_quote! {
                #[allow(unused_macros)]
                macro_rules! spawn {
                    ($($task:ident)::+($($args:expr),*)) => {
                        __lace_task_worker.spawn($($task)::+, ($($args),*))
                    }
                }
            },
            syn::parse_quote! {
                #[allow(unused_macros)]
                macro_rules! sync {
                    ($token:ident) => {
                        __lace_task_worker.sync($token)
                    }
                }
            },
            syn::parse_quote! {
                #[allow(unused_macros)]
                macro_rules! join {
                    ($task_a:ident($($args_a:expr),*), $task_b:ident($($args_b:expr),*)) => {{
                        #[allow(unused_parens)]
                        __lace_task_worker.join( $task_a, ($($args_a),*), $task_b, ($($args_b),*))
                    }}
                }
            },
        ],
    );

    let mut output = TokenStream2::new();
    func.to_tokens(&mut output);
    // eprintln!("{output}");
    output.into()
}
