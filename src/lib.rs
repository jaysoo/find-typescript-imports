#[macro_use]
extern crate napi_derive;
extern crate swc_common;
extern crate swc_ecma_parser;
extern crate swc_ecma_visit;
extern crate swc_ecma_ast;

use futures::stream::StreamExt;
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
  EsConfig
};
use swc_ecma_visit::{Visit};
use swc_ecma_ast::{ImportDecl, Invalid};
use swc_common::DUMMY_SP;

struct ImportVisitor {
    pub imports: Vec<ImportDecl>,
}

impl Visit for ImportVisitor {
    fn visit_import_decl(&mut self, import_decl: &ImportDecl) {
        self.imports.push(import_decl.clone());
    }
}

async fn process_file(file_path: String) -> Vec<String> {
    let content = read_to_string(&file_path).await;
    match content {
      Ok(value) => {
        let cm: Lrc<SourceMap> = Default::default();
        let fm = cm.new_source_file(FileName::Real(file_path.into()), value);

        let lexer = Lexer::new(
          Syntax::Es(EsConfig {
            jsx: true,
            ..EsConfig::default()
          }),
          Default::default(),
          StringInput::from(&*fm),
          None,
        );

        let mut parser = Parser::new_from(lexer);

        let module = parser.parse_module().unwrap();

        let mut visitor = ImportVisitor { imports: vec![] };
        module.visit_module(&Invalid { span: DUMMY_SP }, &mut visitor);

        let mut file_imports: Vec<String> = Vec::new();
        for import in visitor.imports {
            file_imports.push(format!("{:?}", import));
        }
        file_imports
      },
      Err(_) => Vec::new()
    }
}

#[napi]
async fn find_imports(file_paths: Vec<String>) -> Vec<String>  {
  let mut file_reads: futures::stream::FuturesUnordered<_> = file_paths.into_iter().map(process_file).collect();
      
  let mut imports : Vec<Vec<String>> = Vec::new();

  while let Some(result) = file_reads.next().await {
    imports.push(result)
  }

  imports.into_iter().flatten().collect()
}
