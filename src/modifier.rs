mod frame;

use std::fmt::Display;

use crate::{
    data::{ProgramData, WorkspaceData},
    image::ImageOperation,
};

use frame::{Frame, FrameMessage};
use iced::{Command, Element, Renderer};
use iced_native::image::Handle;

/// Trait for modifiers to implement
///
/// Technically not needed due to modifiers being used primarly through enums but helpful in standarizing what functionality they need to support
pub trait Modifier {
    /// The message type expected by this modifier
    type Message: 'static + Into<ModifierMessage> + From<ModifierMessage>;

    /// This function is called to provide image operation of the modifier to be applied to the image in rendering process
    fn get_image_operation(&self, pdata: &ProgramData, wdata: &WorkspaceData) -> ModifierOperation;

    /// Creates a new modifier
    fn create(pdata: &ProgramData, wdata: &WorkspaceData) -> (Command<Self::Message>, Self);

    /// Label of the modifier to be shown in the UI
    fn label() -> &'static str;

    /// Tests whatever the modifier data has been changed in a way that requires redrawing the image
    fn is_dirty(&self) -> bool;

    /// This function is used as a signal to the modifier that it should reset its dirty status
    fn set_clean(&mut self);

    /// This function provides UI that is to be rendered in the main workspace preview area
    ///
    /// # Parameters
    /// `image` - The image that is to be rendered in the preview, can be ignored if the modifier doesn't need it.
    ///
    /// This function should only be used if the modifier needs larger UI area for its operations than properties view provide.
    ///
    /// The function is only called on the currently selected modifier and only if `wants_main_view` function returns true
    #[allow(unused_variables)]
    fn main_view(
        &self,
        image: Handle,
        pdata: &ProgramData,
        wdata: &WorkspaceData,
    ) -> Element<Self::Message, Renderer> {
        iced::widget::image(image).into()
    }

    /// This function allows the modifier to signal that it wants to use the main preview area to draw custom UI
    ///
    /// The function is only called on the currently selected modifier
    #[allow(unused_variables)]
    fn wants_main_view(&self, pdata: &ProgramData, wdata: &WorkspaceData) -> bool {
        false
    }

    /// Optional UI elements to drive properties of the modifier
    #[allow(unused_variables)]
    fn properties_view(
        &self,
        pdata: &ProgramData,
        wdata: &WorkspaceData,
    ) -> Option<Element<Self::Message, Renderer>> {
        None
    }

    /// Message sent as a result of modifier's UI widgets
    #[allow(unused_variables)]
    fn properties_update(
        &mut self,
        message: Self::Message,
        pdata: &mut ProgramData,
        wdata: &mut WorkspaceData,
    ) -> Command<Self::Message> {
        Command::none()
    }

    /// Sends an update to the modifier after the workspace data has been modified
    ///
    /// This function allows modifiers to regenerate their properties or perform commands if they depend on workspace data
    #[allow(unused_variables)]
    fn workspace_update(
        &mut self,
        pdata: &ProgramData,
        wdata: &WorkspaceData,
    ) -> Command<Self::Message> {
        Command::none()
    }
}

/// Carrier enum to allow modifiers to provide what kind of operations they need to apply to the image
pub enum ModifierOperation {
    /// Modifier does not modify the image with its current settings
    None,
    /// A single operation identifier
    Single(ImageOperation),
    /// Two operations, order matters
    Double(ImageOperation, ImageOperation),
    /// More than two operations, order matters
    Multiple(Vec<ImageOperation>),
}

impl From<ImageOperation> for ModifierOperation {
    fn from(value: ImageOperation) -> Self {
        ModifierOperation::Single(value)
    }
}
impl From<(ImageOperation, ImageOperation)> for ModifierOperation {
    fn from(value: (ImageOperation, ImageOperation)) -> Self {
        ModifierOperation::Double(value.0, value.1)
    }
}
impl From<Vec<ImageOperation>> for ModifierOperation {
    fn from(value: Vec<ImageOperation>) -> Self {
        ModifierOperation::Multiple(value)
    }
}

make_modifier!(Frame);
make_modifier_message!(FrameMessage);

/// This makro creates `ModifierBox` enum which is responsible for providing polymorphism feature for all modifiers.
/// `ModifierBox` implements convenience functions for use with `Modifier` trait.
///
/// It also creates ModifierTag enum that is used in creating actual modifiers and packing them into ModifierBox wrapper.
macro_rules! make_modifier {
    ($($md:ident), +) => {
        #[derive(Clone, Debug)]
        pub enum ModifierBox {
            $(
                $md($md),
            )+
        }
        impl ModifierBox {
            /// Provides image operation of the boxed modifier and cleans its dirty status
            pub fn get_image_operation(&mut self, pdata: &ProgramData, wdata: &WorkspaceData) -> ModifierOperation {
                match self {
                    $(
                        ModifierBox::$md(x) => {
                            x.set_clean();
                            x.get_image_operation(pdata, wdata)
                        }
                    )+
                }
            }
            /// Label of the modifier
            pub fn label(&self) -> &'static str {
                match self {
                    $(
                        ModifierBox::$md(_) => $md::label(),
                    )+
                }
            }
            /// Tells whatever the modifier has been changed in a way that needs rerendering of the image
            pub fn is_dirty(&self) -> bool {
                match self {
                    $(
                        ModifierBox::$md(x) => x.is_dirty(),
                    )+
                }
            }
            /// UI for modifier properties
            pub fn properties_view(&self, pdata: &ProgramData, wdata: &WorkspaceData) -> Option<Element<ModifierMessage, Renderer>> {
                match self {
                    $(
                        ModifierBox::$md(x) => match x.properties_view(pdata, wdata) {
                            Some(v) => Some(v.map(|x| x.into())),
                            None => None,
                        },
                    )+
                }
            }
            /// Handles messages sent from modifier UI
            pub fn properties_update(&mut self, mess: ModifierMessage, pdata: &mut ProgramData, wdata: &mut WorkspaceData) -> Command<ModifierMessage> {
                match self {
                    $(
                        ModifierBox::$md(x) => x.properties_update(mess.into(), pdata, wdata).map(|x| x.into()),
                    )+
                }
            }
            /// Signal sent to the modifier that workspace data has changed and the modifier may need to recalculate itself
            pub fn workspace_update(&mut self, pdata: &ProgramData, wdata: &WorkspaceData) -> Command<ModifierMessage> {
                match self {
                    $(
                        ModifierBox::$md(x) => x.workspace_update(pdata, wdata).map(|x| x.into()),
                    )+
                }
            }
            /// UI for the main screen of the workspace for when the modifier needs larger space for specific tasks
            pub fn main_view(&self, image: Handle, pdata: &ProgramData, wdata: &WorkspaceData) -> Element<ModifierMessage, Renderer> {
                match self {
                    $(
                        ModifierBox::$md(x) => x.main_view(image, pdata, wdata).map(|x| x.into()),
                    )+
                }
            }
            /// Tests whatever the modifier wants to take over the main workspace preview area UI
            pub fn wants_main_view(&self, pdata: &ProgramData, wdata: &WorkspaceData) -> bool {
                match self {
                    $(
                        ModifierBox::$md(x) => x.wants_main_view(pdata, wdata),
                    )+
                }
            }
        }

        /// Modifier Tag is used to identify modifiers and their type
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum ModifierTag {
            $(
                $md,
            )+
        }
        impl ModifierTag {
            /// List of all modifiers by their tag
            ///
            /// This is used for easily displaying the modifiers in the UI
            pub const ALL: [Self; count!($($md)+)] = [
                $(
                    Self::$md,
                )+
            ];
            /// Creates a modifier from its tag and puts it in the box enum type
            pub fn make_box(&self, pdata: &ProgramData, wdata: &WorkspaceData) -> (Command<ModifierMessage>, ModifierBox) {
                match self {
                    $(
                        Self::$md => {
                            let (command, modifier) = $md::create(pdata, wdata);
                            (command.map(|x| x.into()), ModifierBox::$md(modifier))
                        }
                    )+
                }
            }
        }
        impl Display for ModifierTag {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{}",
                    match self {
                        $(
                            Self::$md => $md::label(),
                        )+
                    }
                )
            }
        }
    };
}

/// This makro creates `ModifierMessage` which is responsible for boxing messages of modifiers
macro_rules! make_modifier_message {
    ($($mess:ident), +) => {
        #[derive(Debug, Clone)]
        pub enum ModifierMessage {
            $(
                None,
                $mess($mess),
            )+
        }
        $(
            impl From<$mess> for ModifierMessage {
                fn from(value: $mess) -> Self {
                    ModifierMessage::$mess(value)
                }
            }
            impl From<ModifierMessage> for $mess {
                fn from(value: ModifierMessage) -> Self {
                    match value {
                        ModifierMessage::$mess(x) => x,
                        _ => panic!("Received a message that this modifier isn't supposed to! \n$mess\n{:?}", value),
                    }
                }
            }
        )+
            // these should allow modifiers to not need a message if they don't use any
        impl From<()> for ModifierMessage {
            fn from(_: ()) -> Self {
                ModifierMessage::None
            }
        }
        impl From<ModifierMessage> for () {
            fn from(_: ModifierMessage) -> Self {
                ()
            }
        }
    };
}

/// Counts amount of arguments passed into it.
/// Useful in supplying constant values to arrays
macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}
pub(crate) use count;
pub(crate) use make_modifier;
pub(crate) use make_modifier_message;
