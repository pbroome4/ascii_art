use image::codecs::gif::{GifDecoder};
use image::imageops::FilterType::{self, Nearest};
use image::imageops::{crop_imm, grayscale, resize};
use image::{AnimationDecoder, Frame, GenericImageView, ImageBuffer, Pixel, imageops, DynamicImage};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::vec;
use macroquad::{prelude::*, text};


const ASPECT_RATIO: f32 = 16.0 / 9.0;  // width px / height px
const FONT_ASPECT_RATIO: f32 = 0.4;//3.0 / 5.0; // width char / height char 
const ASCII_COLS: usize = 300;  // width in chars
const ASCII_ROWS: usize = (ASCII_COLS as f32  * FONT_ASPECT_RATIO / ASPECT_RATIO) as usize;
type  AsciiGrid = [[u8; ASCII_ROWS]; ASCII_COLS];


////////////////////////////////////////////////////// 
// Process each gifs/images into vector of grayscale.
// hold current 2d grayscale image as var. 
// loop
//      select random noise as a target
//      pick random gif as a target
//      pick each frame of gif as target.



#[macroquad::main("Test")]
async fn main() {
    let gifs = read_gif_resources();
    if gifs.len() == 0{
        return;
    }

    let gif = &gifs[0];
    let mut frame_n = 0;
    let mut next_update = get_time();
    loop{
        println!("Drawing frame #{}", frame_n);
        let frame = &gif.frames[frame_n % gif.frames.len()];
        let s = grid_to_string(&frame.grid);
        let (x, y, font_size) = calc_multiline_text_dim(&s);
        clear_background(BLACK);
        draw_multiline_text(s, x, y, font_size, Some(1.0), WHITE);
        if get_time() > next_update{
            frame_n += 1;
            next_update = get_time() + frame.delay.as_secs_f64();   
        }
    
        next_frame().await
    }
}

fn calc_multiline_text_dim(s: &str) -> (f32, f32, f32){
        let (w_g, h_g) = grid_screen_dim(); // assumes size 1 font
        let scale = (screen_width() / w_g).max(screen_height() / h_g).ceil();
        let (x, y) = calc_position(scale*w_g, scale*h_g);
        return (x,y,scale);
}

// Calculates the x0, y0 for top left corner of a rect to center it on the screen
// provided the width and height of the rectangle 
fn calc_position(w_img: f32, h_img: f32) -> (f32, f32){
    let x0 = screen_width() / 2.0 - w_img / 2.0;
    let y0 = screen_height() / 2.0 - h_img / 2.0;
    return (x0, y0);
}

struct AsciiFrame{
    grid: AsciiGrid,
    delay: Duration,  // seconds
}

impl Default for AsciiFrame{
    fn default() -> Self {
        Self {
            grid: [[0; ASCII_ROWS]; ASCII_COLS],
            delay: Duration::from_millis(2000),
        }
    }
}


struct AsciiGif{
    frames: Vec<AsciiFrame>,
}


impl From<Vec<Frame>> for AsciiGif{
    fn from(frames: Vec<Frame>) -> Self {
        let mut v = Vec::new();
        for frame in frames{
            let mut ascii = AsciiFrame::default();
            ascii.delay = frame.delay().into();
            let im_buf = frame.into_buffer();
            let im_buf = stretch_hw( // predistort the image bc font not square
                &im_buf,
                FONT_ASPECT_RATIO,
                Nearest);
            let im_buf = resize_to_fill( 
                &im_buf,
                ASCII_COLS as u32,
                ASCII_ROWS as u32,
                true,
                Nearest);
            let im_buf = crop_center(
                &im_buf,
                ASCII_COLS as u32,
                ASCII_ROWS as u32
            ); 
            let im_gray_scale = grayscale(&im_buf);
            for (x, y, p) in im_gray_scale.enumerate_pixels(){
                ascii.grid[x as usize][y as usize] = p[0];
            }
            v.push(ascii);
        }
        return Self {
            frames: v
        };
    }
}

impl From<PathBuf> for AsciiGif{
    fn from(value: PathBuf) -> Self {
        let file_in = BufReader::new(File::open(value).unwrap());
        let decoder = GifDecoder::new(file_in).unwrap();
        let frames = decoder.into_frames()
                            .collect_frames()
                            .expect("error decoding gif");
        return AsciiGif::from(frames);
    }
}

fn read_gif_resources() -> Vec<AsciiGif>{
    let mut v = Vec::new();
    println!("Searching for GIF resources");
    let dir_entries  = fs::read_dir("resources/").unwrap();
    for dir_entry in dir_entries{
        let path = dir_entry.unwrap().path();
        let path_str = path.clone().into_string().unwrap();
        if path_str.ends_with(".gif"){
            println!("Processing GIF resource: {}", path_str);
            let a = AsciiGif::from(path);
            v.push(a);
        }
    }
    return v;
}

// Scale image up by hw = height/width.
// If hw > 1, scales height up, width constant
// if hw < 1, scales width up, height constant.
fn stretch_hw<I:GenericImageView>(
    i: &I,
    hw: f32,
    filter_type:FilterType
) -> ImageBuffer<I::Pixel, Vec<<I::Pixel as Pixel>::Subpixel>>
where
    I::Pixel: 'static,
    <I::Pixel as Pixel>::Subpixel: 'static,
{
    if hw < 1.0{
        let new_width = (i.width() as f32 / hw) as u32;
        let r = imageops::resize(i, new_width, i.height() as u32, filter_type);
        return r;
    } 
    let new_height = (i.height() as f32 * hw) as u32;
    let r = imageops::resize(i, i.width() as u32, new_height, filter_type);
    return r;
}


// resize image to fit dimensions h and w while preserving aspect ratio.
// If inner = true, resized image w and h are <= border w and h.
// if inner = false, resized image w and h are >= border w and h.
fn resize_to_fill<I:GenericImageView>(
    i: &I,
    w: u32,
    h: u32,
    inner: bool,
    filter: FilterType
) -> ImageBuffer<I::Pixel, Vec<<I::Pixel as Pixel>::Subpixel>>
where
    I::Pixel: 'static,
    <I::Pixel as Pixel>::Subpixel: 'static,
{
    let h_scale = h as f32 / i.height() as f32;
    let w_scale = w as f32 / i.width() as f32;
    let scale = if inner{ h_scale.min(w_scale) } else{ h_scale.max(w_scale)};
    let new_height = (i.height() as f32 * scale) as u32;
    let new_width = (i.width() as f32 * scale) as u32;
    return resize(i, new_width, new_height, filter);
}

fn crop_center<I:GenericImageView>(
    i: &I,
    w: u32,
    h: u32
) -> ImageBuffer<I::Pixel, Vec<<I::Pixel as Pixel>::Subpixel>>
where
    I::Pixel: 'static,
    <I::Pixel as Pixel>::Subpixel: 'static,
{
    let mut im_buf = imageops::resize(i, i.width(), i.height(), Nearest); // TODO how else to convert image to imagebuf?
    if w > im_buf.width(){
        println!("Warning, crop width larger than image width, resizing image instead");
        im_buf = resize(&im_buf, w, im_buf.height(), Nearest);
    }
    if h > im_buf.height(){
        println!("Warning, crop height is larger than image height, resizing image instead");
        im_buf = resize(&im_buf, im_buf.width(), h, Nearest);
    }
    let x0 = im_buf.width()/2 - w/2;
    let y0 = im_buf.height()/2 - h/2;
    let im_buf  = imageops::crop_imm(&im_buf, x0, y0, w, h).to_image();
    return im_buf;
}


// Converts grayscale value to ascii char
// from value from 0 (empty) to 255 (Filled)
fn get_ascii_char(gray: u8, reverse: bool) -> char {
    const RAMP: &[u8] = b".'`^\\\",;:Iel!><~+_-?][}{)(|/tfjxrnuvcXYZUJCQL0OZmwqpdbhkro*#MW&8%B@$";
    let max_index = RAMP.len() - 1;
    let mut index = ((gray as usize) * max_index) / 255;
    if reverse {
        index = max_index - index;
    }
    RAMP[index] as char
}



fn grid_to_string(grid: &AsciiGrid) -> String{
    let mut s = String::new();
    for r in 0..ASCII_ROWS{
        for c in 0..ASCII_COLS{
            s.push(get_ascii_char(grid[c][r], false));
        }
        s.push('\n');
    }

    return s;
}


// Gets width/height of grid drawn to screen in pixels assuming size one font.
// Use this to determine font size.
fn grid_screen_dim() -> (f32, f32){
    let text_dimensions = measure_multiline_text( // Measure size of macroquad font
        &grid_to_string(&AsciiFrame::default().grid),
        None,
        1,
        1.0,
        Some(1.0));
    return (text_dimensions.width, text_dimensions.height);
        
}
