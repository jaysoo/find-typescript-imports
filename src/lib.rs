#[macro_use]
extern crate napi_derive;
extern crate futures;
extern crate tokio;
extern crate swc_common;
extern crate swc_ecma_parser;
extern crate swc_ecma_visit;
extern crate swc_ecma_ast;

use futures::future::join_all;
use std::io::{Error, ErrorKind};
use tokio::fs::read_to_string;
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
use swc_ecma_ast::{ImportDecl};

struct ImportVisitor {
  pub imports: Vec<String>,
}

impl Visit for ImportVisitor {
  fn visit_import_decl(&mut self, import_decl: &ImportDecl) {
    self.imports.push(import_decl.src.value.to_string());
  }
}

async fn process_file(file_path: String) -> Vec<String> {
  let tsx = file_path.ends_with(".tsx");
  let content = read_to_string(&file_path).await;

  let result = content
    .and_then(|source| {
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
      let mut visitor = ImportVisitor { imports: vec![] };
      let module = parser.parse_module()
        .map_err(|_| Error::new(ErrorKind::Other, "Failed to parse file"))?;

      module.visit_with(&mut visitor);

      Ok(visitor.imports)
    });

  match result {
    Ok(value) => value,
    Err(_) => vec![]
  }
}

#[napi]
async fn find_imports(file_paths: Vec<String>) -> Vec<String>  {
  let futures: Vec<_> = file_paths.into_iter().map(process_file).collect();
  let results: Vec<_> = join_all(futures).await;
  results.into_iter().flatten().collect()
}
