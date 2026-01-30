use iced::{
    widget::{column as col, container, row, text, text_input},
    Alignment, Command, Element, Length, Renderer, Vector,
};
use iced_native::image::Handle;
use image::{Pixel, Rgba};

use crate::{
    data::{
        has_invalid_characters, sanitize_file_name, sanitize_file_name_allow_path, FrameImage,
        ProgramData,
    },
    image::{convert::image_to_handle, operations::flood_fill_mask, GrayscaleImage, RgbaImage},
    style::Style,
    widgets::PixelSampler,
};

/// Editor for creating new frames for use in the program
pub struct FrameMaker {
    /// Name for the new frame image
    name: String,
    /// Name for the category this frame should be added to
    category: String,
    /// Preview to display in the editor
    preview: Handle,
    /// Actual image of the frame
    frame: RgbaImage,
    /// The grayscale mask image this editor is meant to help create
    mask: Option<GrayscaleImage>,
    /// Flag that marks whatever the editor is awaiting rendering result
    rendering: bool,
}

#[derive(Debug, Clone)]
pub enum FrameMakerMessage {
    /// Result of user clicking the PixelSampler
    /// The vector is pixel location the user clicked
    SelectedPixel(Vector<u32>),
    /// Result of generating the frame, gives preview image and the mask
    GeneratedMask(Handle, GrayscaleImage),
    /// Sets the name for the frame
    SetName(String),
    /// Sets the category for the frame
    SetCategory(String),
}

impl FrameMaker {
    /// Creates empty `FrameMaker` editor. You need to call `load` function on it to initialize it
    pub fn new() -> Self {
        // Create dummy image
        let image = RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
        Self {
            name: String::from("frame"),
            category: String::from("frame"),
            mask: None,
            preview: image_to_handle(image.clone()),
            frame: image,
            rendering: false,
        }
    }

    /// Changes name displayed in the editor
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Loads provided image into the editor, making it ready for display
    pub fn load(&mut self, frame: RgbaImage) {
        self.name = String::from("new-frame");
        self.category = String::from("frame");
        self.mask = None;
        self.preview = image_to_handle(frame.clone());
        self.frame = frame;
    }

    /// Exports editor result to a `FrameImage`
    ///
    /// # Panics
    /// If the `can_save` test result is false, this function will likely throw a panic
    pub fn create_frame(&self) -> FrameImage {
        FrameImage::new(
            self.name.clone(),
            self.category.clone(),
            self.frame.clone(),
            self.mask.clone(),
        )
    }

    /// Tests if the frame can be saved
    pub fn can_save(&self) -> bool {
        // must have the mask to save
        if self.mask.is_none() {
            return false;
        }
        // have to wait to finish rendering
        if self.rendering {
            return false;
        }
        // TODO test if the path is taken or not
        true
    }

    /// Constructs UI for the editor
    pub fn view(&self, _pdata: &ProgramData) -> Element<'_, FrameMakerMessage, Renderer> {
        let name = row![
            text("Name: "),
            text_input(
                "New Frame Name",
                &self.name,
                |x| FrameMakerMessage::SetName(x)
            ),
        ]
        .spacing(5)
        .padding(5)
        .align_items(Alignment::Center)
        .height(Length::Shrink);

        let category = row![
            text("Category: "),
            text_input("Category for new frame", &self.category, |x| {
                FrameMakerMessage::SetCategory(x)
            }),
        ]
        .spacing(5)
        .padding(5)
        .align_items(Alignment::Center)
        .height(Length::Shrink);

        let name = container(name).style(Style::Frame);
        let category = container(category).style(Style::Frame);

        let preview = container(PixelSampler::new(self.preview.clone(), |x| {
            FrameMakerMessage::SelectedPixel(x)
        }))
        .style(Style::Margins)
        .center_x()
        .center_y()
        .width(Length::Fill)
        .height(Length::Fill);

        let ui = col![name, category, preview,].spacing(2).padding(2);

        container(ui).style(Style::Margins).into()
    }

    /// Handles messages produced by the editor
    pub fn update(
        &mut self,
        message: FrameMakerMessage,
        pdata: &mut ProgramData,
    ) -> Command<FrameMakerMessage> {
        match message {
            FrameMakerMessage::SelectedPixel(p) => {
                self.rendering = true;
                Command::perform(create_mask(self.frame.clone(), p), |(h, g)| {
                    FrameMakerMessage::GeneratedMask(h, g)
                })
            }
            FrameMakerMessage::GeneratedMask(image, mask) => {
                self.rendering = false;
                self.mask = Some(mask);
                self.preview = image;
                Command::none()
            }
            FrameMakerMessage::SetName(n) => {
                if has_invalid_characters(&n) {
                    pdata
                        .status
                        .warning("Removed invalid characters from frame name")
                }
                self.name = sanitize_file_name(n);
                Command::none()
            }
            FrameMakerMessage::SetCategory(n) => {
                if has_invalid_characters(&n) {
                    pdata
                        .status
                        .warning("Removed invalid characters from frame category")
                }
                self.category = sanitize_file_name_allow_path(n);
                Command::none()
            }
        }
    }
}

/// Creates a mask out of the image by flood spreading the mask pixel by pixel from the source position using alpha channel of the image.
async fn create_mask(image: RgbaImage, flood_source: Vector<u32>) -> (Handle, GrayscaleImage) {
    let mask = flood_fill_mask(&image, flood_source, 0, |s| {
        if s[3] < 255 {
            Some(255)
        } else {
            None
        }
    });

    let width = image.width() as usize;

    // calculates linear index of a pixel
    macro_rules! index {
        ($x:expr, $y:expr) => {
            width * $y + $x
        };
    }

    // creates a grid pattern and overlays the frame image onto it
    let masked_area = RgbaImage::from_fn(image.width(), image.height(), |x, y| {
        let mask = mask.as_raw();
        let a = mask[index!(x as usize, y as usize)];
        let pixel = *image.get_pixel(x, y);
        if a == 0 {
            return pixel;
        }
        let grid_x = x % 100;
        let grid_y = y % 100;
        let mut dark = if grid_x >= 50 { true } else { false };
        if grid_y >= 50 {
            dark = !dark;
        }
        let mut grid: Rgba<u8> = if dark {
            [128, 128, 128, a].into()
        } else {
            [158, 158, 158, a].into()
        };
        grid.blend(&pixel);
        grid
    });
    let handle = image_to_handle(masked_area);
    (handle, mask)
}
