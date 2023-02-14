

use iced::{Command, Element, Renderer};
use image::DynamicImage;


/// Trait for modifiers to implement, technically not needed but helpful in standarizing what functionality they need to support
pub trait Modifier {
    type Message: 'static + Into<ModifierMessage> + From<ModifierMessage>;

    fn modify(&self, image: DynamicImage) -> DynamicImage;
    fn label(&self) -> &str;
    fn properties_view(&self) -> Option<Element<Self::Message, Renderer>> {
        None
    }
    #[allow(unused_variables)]
    fn properties_update(&mut self, message: Self::Message) -> Command<Self::Message> {
        Command::none()
    }
}

make_modifier!(Frame);
make_modifier_message!(FrameMessage);

/// This makro creates `ModifierBox` enum which is responsible for providing polymorphism feature for all modifiers.
/// `ModifierBox` implements convenience functions for use with `Modifier` trait.
macro_rules! make_modifier {
    ($($md:ident), +) => {
        #[derive(Clone, Debug)]
        pub enum ModifierBox {
            $(
                $md($md),
            )+
        }
        impl ModifierBox {
            pub fn modify(&self, image: DynamicImage) -> DynamicImage {
                match self {
                    $(
                        ModifierBox::$md(x) => x.modify(image),
                    )+
                }
            }
            pub fn label(&self) -> &str {
                match self {
                    $(
                        ModifierBox::$md(x) => x.label(),
                    )+
                }
            }
            pub fn properties_view(&self) -> Option<Element<ModifierMessage, Renderer>> {
                match self {
                    $(
                        ModifierBox::$md(x) => match x.properties_view() {
                            Some(v) => Some(v.map(|x| x.into())),
                            None => None,
                        },
                    )+
                }
            }
            pub fn properties_update(&mut self, mess: ModifierMessage) -> Command<ModifierMessage> {
                match self {
                    $(
                        ModifierBox::$md(x) => x.properties_update(mess.into()).map(|x| x.into()),
                    )+
                }
            }
        }
        $(
            impl TryFrom<ModifierBox> for $md {
                type Error = ();
                fn try_from(value: ModifierBox) -> Result<Self, Self::Error> {
                    match value {
                        ModifierBox::$md(x) => Ok(x),
                        _ => Err(())
                    }
                }
            }
            impl From<$md> for ModifierBox {
                fn from(value: $md) -> Self {
                    ModifierBox::$md(value)
                }
            }
        )+
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
pub(crate) use make_modifier;
pub(crate) use make_modifier_message;
