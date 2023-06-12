#[macro_use]
extern crate napi_derive;
extern crate futures;
extern crate tokio;
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
  TsConfig
};
use swc_ecma_visit::{Visit,VisitWith};
use swc_ecma_ast::{ImportDecl};

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
    let file_path_2 = file_path.clone();
    match content {
      Ok(source) => {
        let cm: Lrc<SourceMap> = Default::default();
        let fm = cm.new_source_file(FileName::Real(file_path.into()), source.clone());

        let lexer = Lexer::new(
          Syntax::Typescript(TsConfig {
            tsx: false,
            ..TsConfig::default()
          }),
          Default::default(),
          StringInput::from(&*fm),
          None,
        );

        let mut parser = Parser::new_from(lexer);

        let result = parser.parse_module();
        match result {
          Ok(module) => {
            let mut visitor = ImportVisitor { imports: vec![] };
            module.visit_with(&mut visitor);

            let mut file_imports: Vec<String> = vec![];
            for import_decl in visitor.imports {
                let span = import_decl.span;
              let start = (span.lo().0 - 1) as usize;
              let end = span.hi().0 as usize;
              let import_text = source[start..end].to_string();
              file_imports.push(format!("{}: {}", file_path_2, import_text));
            }
            file_imports
          },
          Err(_) =>  vec![]
        }
      },
      Err(_) => vec![]
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
