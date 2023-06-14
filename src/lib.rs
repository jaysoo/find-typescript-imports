#[macro_use]
extern crate napi_derive;

use rayon::prelude::*;
use std::io::{Error, ErrorKind};
use std::fs::read_to_string;
use swc_common::{
    sync::Lrc,
    FileName,
    SourceMap,
};
use swc_ecma_parser::{
    lexer::Lexer,
    Parser,
    Syntax,
    StringInput,
    TsConfig
};
use swc_ecma_visit::{Visit,VisitWith};
use swc_ecma_ast::{ImportDecl, CallExpr, Expr, Lit, Str};
use napi::{JsFunction};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{
    ThreadSafeCallContext,
    ThreadsafeFunction,
    ThreadsafeFunctionCallMode,
};

#[napi]
pub struct ImportResult {
  pub file: String,
  pub import_expr: String,
}

struct ImportVisitor<'a> {
    pub file: String,
    pub callback: &'a ThreadsafeFunction<ImportResult>,
}

impl Visit for ImportVisitor<'_> {
    fn visit_import_decl(&mut self, import_decl: &ImportDecl) {
        self.callback.call(Ok(ImportResult {
          file: self.file.to_owned(),
          import_expr: import_decl.src.value.to_string(),
        }), ThreadsafeFunctionCallMode::NonBlocking);
    }

    fn visit_call_expr(&mut self, call_expr: &CallExpr) {
      if call_expr.callee.is_import() {
          if let Some(arg) = call_expr.args.get(0) {
              if let Expr::Lit(lit) = &*arg.expr {
                  if let Lit::Str(Str { value: sym, raw: _, span: _ }) = lit {
                    self.callback.call(Ok(ImportResult {
                      file: self.file.to_owned(),
                      import_expr: sym.to_string(),
                    }), ThreadsafeFunctionCallMode::NonBlocking);
                  }
              }
          }
      }
  }
}

fn process_file(file_path: String, callback: &mut ThreadsafeFunction<ImportResult>) -> () {
    let file = file_path.clone();
    let tsx = file_path.ends_with(".tsx") || file_path.ends_with(".jsx");
    let content = read_to_string(&file_path);

    match content {
      Ok(source) => {
        let cm: Lrc<SourceMap> = Default::default();
        let fm = cm.new_source_file(FileName::Real(file_path.into()), source.clone());
        let lexer = Lexer::new(
            Syntax::Typescript(TsConfig {
                tsx,
                ..TsConfig::default()
            }),
            Default::default(),
            StringInput::from(&*fm),
            None,
        );
        let mut parser =  Parser::new_from(lexer);
        let mut visitor = ImportVisitor { file, callback };
        let module = parser.parse_module()
            .map_err(|_| Error::new(ErrorKind::Other, "Failed to parse file"));

        match module {
            Ok(module) => {
              module.visit_with(&mut visitor);
            },
            Err(_) => {}
        }
      },
      Err(_) => {}
    }
}

#[napi]
fn find_imports(
    file_paths: Vec<String>,
    #[napi(ts_arg_type = "(obj: null, result: {file: string, importExpr: string}) => void")]
    callback: JsFunction
) -> Result<()> {
    let thread_safe_callback: ThreadsafeFunction<ImportResult> =
      callback.create_threadsafe_function(
          0,
          |ctx: ThreadSafeCallContext<ImportResult>| {
            let data: ImportResult = ctx.value;
            Ok(vec![data])
          },
      )?;

    file_paths
      .into_par_iter()
      .for_each_with(thread_safe_callback, |callback, file_path| {
          process_file(file_path, callback)
      });

  Ok(())
}
