//! Adapted from <https://github.com/YarnSpinnerTool/YarnSpinner/blob/da39c7195107d8211f21c263e4084f773b84eaff/YarnSpinner/Dialogue.cs>, which we split off into multiple files
//!
//! ## Implementation notes
//!
//! - `OptionSet` was replaced by a simple `Vec<DialogueOption>`
//! - Additional newtypes were introduced for strings.

use crate::prelude::*;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

/// A command, sent from the [`Dialogue`] to the game.
///
/// You do not create instances of this struct yourself. They are created by the [`Dialogue`] during program execution.
///
/// ## See also
/// [`CommandHandler`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command(pub String);
impl Deref for Command {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Command {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeName(pub String);
impl Deref for NodeName {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for NodeName {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

macro_rules! impl_function_newtype_with_no_params {
    ($(#[$attr:meta])* pub struct $struct_name:ident(pub $trait_name:ident: FnMut())) => {
        $(#[$attr])*
        #[derive(Debug, Clone)]
        pub struct $struct_name(pub Box<dyn $trait_name + Send + Sync>);

        impl Clone for Box<dyn $trait_name + Send + Sync> {
            fn clone(&self) -> Self {
                self.clone_box()
            }
        }

        impl Debug for dyn $trait_name + Send + Sync {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, stringify!($struct_name))
            }
        }

        pub trait $trait_name {
            fn call(&mut self);
            fn clone_box(&self) -> Box<dyn $trait_name + Send + Sync>;
        }

        impl<T> $trait_name for T
        where
            T: FnMut() + Clone + Send + Sync + 'static,
        {
            fn call(&mut self) {
                self()
            }

            fn clone_box(&self) -> Box<dyn $trait_name + Send + Sync> {
                Box::new(self.clone())
            }
        }
    };
}

macro_rules! impl_function_newtype {
    ($(#[$attr:meta])* pub struct $struct_name:ident(pub $trait_name:ident: FnMut($param:ty))) => {
        $(#[$attr])*
        #[derive(Debug, Clone)]
        pub struct $struct_name(pub Box<dyn $trait_name + Send + Sync>);

        impl Deref for $struct_name {
            type Target = Box<dyn $trait_name + Send + Sync>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $struct_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl Clone for Box<dyn $trait_name + Send + Sync> {
            fn clone(&self) -> Self {
                self.clone_box()
            }
        }

        impl Debug for dyn $trait_name + Send + Sync {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, stringify!($struct_name))
            }
        }

        pub trait $trait_name: Send + Sync {
            fn call(&mut self, param: $param);
            fn clone_box(&self) -> Box<dyn $trait_name + Send + Sync>;
        }

        impl<T> $trait_name for T
        where
            T: FnMut($param) + Clone + Send + Sync + 'static,
        {
            fn call(&mut self, param: $param) {
                self(param)
            }

            fn clone_box(&self) -> Box<dyn $trait_name + Send + Sync> {
                Box::new(self.clone())
            }
        }
    };
}

impl_function_newtype! {
    /// Represents a method that receives diagnostic messages and error information from a [`Dialogue`].
    ///
    /// The text that this delegate receives may be output to a console, or sent to a log.
    ///
    /// ## Params
    /// - The text that should be logged.
    pub struct Logger(pub LoggerFn: FnMut(String))
}

impl_function_newtype! {
    /// Represents the method that is called when the [`Dialogue`] delivers a [`Line`].
    ///
    /// ## See also
    /// - [`OptionsHandler`]
    /// - [`CommandHandler`]
    /// - [`NodeStartHandler`]
    /// - [`NodeCompleteHandler`]
    /// - [`DialogueCompleteHandler`]
    pub struct LineHandler(pub LineHandlerFn: FnMut(Line))
}

impl_function_newtype! {
    /// Represents the method that is called when the [`Dialogue`] delivers an [`OptionSet`].
    ///
    /// ## See also
    /// - [`LineHandler`]
    /// - [`CommandHandler`]
    /// - [`NodeStartHandler`]
    /// - [`NodeCompleteHandler`]
    /// - [`DialogueCompleteHandler`]
    pub struct OptionsHandler(pub OptionsHandlerFn: FnMut(Vec<DialogueOption>))
}

impl_function_newtype! {
    /// Represents the method that is called when the [`Dialogue`] delivers a [`Command`].
    ///
    /// ## See also
    /// - [`LineHandler`]
    /// - [`OptionsHandler`]
    /// - [`NodeStartHandler`]
    /// - [`NodeCompleteHandler`]
    /// - [`DialogueCompleteHandler`]
    pub struct CommandHandler(pub CommandHandlerFn: FnMut(Command))
}

impl_function_newtype! {
    /// Represents the method that is called when the [`Dialogue`] reaches the end of a node.
    ///
    /// This method may be called multiple times over the course of code execution. A node being complete does not necessarily represent the end of the conversation.
    ///
    /// ## See also
    /// - [`LineHandler`]
    /// - [`OptionsHandler`]
    /// - [`CommandHandler`]
    /// - [`NodeStartHandler`]
    /// - [`DialogueCompleteHandler`]
    pub struct NodeCompleteHandler(pub NodeCompleteHandlerFn: FnMut(NodeName))
}

impl_function_newtype! {
    /// Represents the method that is called when the [`Dialogue`] begins executing a node.
    ///
    /// ## See also
    /// - [`LineHandler`]
    /// - [`OptionsHandler`]
    /// - [`CommandHandler`]
    /// - [`NodeCompleteHandler`]
    /// - [`DialogueCompleteHandler`]
    pub struct NodeStartHandler(pub NodeStartHandlerFn: FnMut(NodeName))
}

impl_function_newtype_with_no_params! {
    /// Represents the method that is called when the dialogue has reached its end, and no more code remains to be run.
    ///
    /// ## See also
    /// - [`LineHandler`]
    /// - [`OptionsHandler`]
    /// - [`CommandHandler`]
    /// - [`NodeStartHandler`]
    /// - [`NodeCompleteHandler`]
    pub struct DialogueCompleteHandler(pub DialogueCompleteHandlerFn: FnMut())
}

impl_function_newtype! {
    /// Represents the method that is called when the dialogue anticipates that it will deliver lines.
    ///
    /// This method should begin preparing to run the lines. For example, if a game delivers dialogue via voice-over,
    /// the appropriate audio files should be loaded.
    ///
    /// This method serves to provide a hint to the game that a line _may_ be run.
    /// Not every line indicated in the provided `LineId`s may end up actually running.
    ///
    /// This method may be called any number of times during a dialogue session.
    pub struct PrepareForLinesHandler(pub PrepareForLinesHandlerFn: FnMut(Vec<LineId>))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_assign_handlers() {
        let _logger = Logger(Box::new(|message| println!("{}", message)));

        let _dialogue_complete_handler =
            DialogueCompleteHandler(Box::new(|| println!("Dialogue complete!")));
    }
}