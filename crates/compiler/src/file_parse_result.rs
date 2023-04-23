//! Adapted from <https://github.com/YarnSpinnerTool/YarnSpinner/blob/da39c7195107d8211f21c263e4084f773b84eaff/YarnSpinner.Compiler/FileParseResult.cs>

use crate::error_strategy::ErrorStrategy;
use crate::parser::generated::yarnspinnerparser::{
    DialogueContextAll, YarnSpinnerParserContextType,
};
use crate::prelude::generated::yarnspinnerlexer::YarnSpinnerLexer;
use crate::prelude::generated::yarnspinnerparser::YarnSpinnerParser;
use antlr_rust::common_token_stream::CommonTokenStream;

use antlr_rust::InputStream;
use std::rc::Rc;

/// Contains the result of parsing a single file of source code.
///
/// This class provides only syntactic information about a parse - that is,
/// it provides access to the parse tree, and the stream of tokens used to
/// produce that parse tree.
pub struct FileParseResult<'input> {
    pub name: String,

    pub tree: Rc<DialogueContextAll<'input>>,

    /// This was not in the original, but in Rust we need to actually store
    /// the parser itself somewhere, which is why we store it here.
    /// We also end up leading the `ErrorStrategy` into the public interface, but using generics here makes
    /// the code a lot more complicated without actually providing much benefit.
    _parser: YarnSpinnerParser<
        'input,
        CommonTokenStream<'input, YarnSpinnerLexer<'input, InputStream<&'input [u8]>>>,
        ErrorStrategy<'input, YarnSpinnerParserContextType>,
    >,
}

impl<'input> FileParseResult<'input> {
    pub fn new(
        name: String,
        mut parser: YarnSpinnerParser<
            'input,
            CommonTokenStream<'input, YarnSpinnerLexer<'input, InputStream<&'input [u8]>>>,
            ErrorStrategy<'input, YarnSpinnerParserContextType>,
        >,
    ) -> Self {
        Self {
            name,
            tree: parser.dialogue().unwrap(),
            _parser: parser,
        }
    }
}