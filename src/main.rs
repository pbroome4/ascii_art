use image::codecs::gif::{GifDecoder};
use image::imageops::FilterType::{self, Nearest};
use image::imageops::{grayscale, resize};
use image::{AnimationDecoder, Frame, GenericImageView, ImageBuffer, Pixel, imageops};
use macroquad::miniquad::date::now;
use macroquad::rand::{RandomRange};
use std::clone;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{PathBuf};
use macroquad::{prelude::*};
use std::ops::{Add, Index, IndexMut, Mul};
use std::time::{Duration, Instant};

use crate::DisplayStates::Blur;


const ASPECT_RATIO: f32 = 16.0 / 9.0;  // width px / height px
const FONT_ASPECT_RATIO: f32 = 0.4;//3.0 / 5.0; // width char / height char 
const ASCII_COLS: usize = 500;  // width in chars
const ASCII_ROWS: usize = (ASCII_COLS as f32  * FONT_ASPECT_RATIO / ASPECT_RATIO) as usize;


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


    let blur_dur = Duration::from_secs(3); // seconds
    let focus_duration = Duration::from_secs(5); // seconds
    let mut curr_gif = &gifs[0];
    let mut curr_grid = AsciiGrid::default();
    let mut curr_state = DisplayStates::Blank;
    let mut last_state_update = Instant::now();
    let mut should_update_frame = true;

    let mut blur_grid_start = AsciiGrid::default();
    let mut blur_grid_end = AsciiGrid::default();

    let mut focus_grid_start = AsciiGrid::default();
    let mut focus_grid_end = AsciiGrid::default();
    
    let mut play_frames = curr_gif.frames.iter();
    let mut play_frame = play_frames.next().unwrap();
    let mut last_play_update = Instant::now();

    loop{
        match curr_state{
            DisplayStates::Blank =>{
                curr_state = DisplayStates::Blur;
                curr_grid = AsciiGrid::default();
                blur_grid_start = curr_grid.clone();
                blur_grid_end = AsciiGrid::random();
                should_update_frame = true;
            }
            DisplayStates::Blur => {
                let mut i = (Instant::now() - last_state_update).as_secs_f64() / blur_dur.as_secs_f64();
                i = i.min(1.0);
                curr_grid = blur_grid_start.add_eps(&blur_grid_end, i as f32);
                should_update_frame = true;
                
                if i >= 1.0 {
                    curr_state = DisplayStates::Focus;
                    let gif_index = RandomRange::gen_range(0, gifs.len());
                    info!("Selecting index: {}", gif_index);
                    curr_gif = &gifs[gif_index];
                    focus_grid_start = curr_grid.clone();
                    focus_grid_end = curr_gif.frames[0].grid.clone();
                    last_state_update = Instant::now();
                }
            }

            DisplayStates::Focus => {
                let mut i = (Instant::now() - last_state_update).as_secs_f64() / focus_duration.as_secs_f64();
                i = i.min(1.0);
                curr_grid = focus_grid_start.add_eps(&focus_grid_end, i as f32);
                should_update_frame = true;
                if i >= 1.0{
                    curr_state = DisplayStates::Play;
                    play_frames = curr_gif.frames.iter();
                    last_play_update = Instant::now();
                    last_state_update =Instant::now();
                }
            }

            DisplayStates::Play => {                
                if Instant::now() - last_play_update > play_frame.delay{
                    if let Some(next_play_frame) = play_frames.next(){
                        play_frame = next_play_frame;
                        curr_grid = play_frame.grid.clone();
                        should_update_frame = true;
                    }else{
                        blur_grid_start = curr_grid.clone();
                        blur_grid_end = AsciiGrid::random();
                        curr_state = Blur;
                        last_state_update = Instant::now();
                    }
                }
            }
        }
        let a = AsciiScreenParams::from(&curr_grid);
        clear_background(BLACK);
        draw_multiline_text(a.s, a.x, a.y, a.font_size, Some(1.0), WHITE);    
        next_frame().await
    }
}

enum DisplayStates{
    Blank,
    Blur,
    Focus,
    Play
}

#[derive(Clone)]
struct AsciiGrid{
    g: [[u8; ASCII_ROWS]; ASCII_COLS]
} 

impl AsciiGrid{
    fn random()->Self{
        let mut a = AsciiGrid::default();
        for c in 0..ASCII_COLS{
            for r in 0..ASCII_ROWS{
                a[c][r] = RandomRange::gen_range(0, 255) as u8;
            }
        }
        return a;
    }

    //fn blur(&mut self, epsilon: f32){
    //    let rnd = AsciiGrid::random();
    //    for c in 0..ASCII_COLS{
    //        for r in 0..ASCII_ROWS{
    //            let v =  (1.0 - epsilon) * self[c][r] as f32 +
    //                         epsilon * rnd[c][r] as f32;
    //            self[c][r] = v as u8;
    //        }
    //    }
    //}

    fn add_eps(&self, other: &AsciiGrid, epsilon: f32)-> Self{
        let mut ret = AsciiGrid::default();
        for c in 0..ASCII_COLS{
            for r in 0..ASCII_ROWS{
                let v =  (1.0 - epsilon) * self[c][r] as f32 +
                             epsilon * other[c][r] as f32;
                ret[c][r] = v as u8;
            }
        }
        return ret;
    }
}

impl Default for AsciiGrid{
    fn default() -> Self {
        Self { g: [[0;ASCII_ROWS]; ASCII_COLS] }
    }
}

//impl Add<&Self> for AsciiGrid{
//    type Output = Self;
//    fn add(mut self, rhs: &Self) -> Self::Output {
//        for c in 0..ASCII_COLS{
//            for r in 0..ASCII_ROWS{
//                self.g[c][r] += rhs.g[c][r];
//            }
//        }
//        return self;
//    }
//}

impl Index<usize> for AsciiGrid{
    type Output = [u8; ASCII_ROWS];
    fn index(&self, index: usize) -> &Self::Output {
        return &self.g[index];
    }
}


impl IndexMut<usize> for AsciiGrid{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        return &mut self.g[index];
    }
}


impl From<&AsciiGrid> for String{
    fn from(g: &AsciiGrid) -> Self {
    let mut s = String::new();
    for r in 0..ASCII_ROWS{
        for c in 0..ASCII_COLS{
            s.push(get_ascii_char(g[c][r], false));
        }
        s.push('\n');
    }
    return s;   
    }
}


struct AsciiScreenParams{
    s: String,
    x: f32,
    y: f32,
    font_size: f32
}


impl From<&AsciiGrid> for AsciiScreenParams{
    fn from(g: &AsciiGrid) -> Self {    
        let s = String::from(g);
        let (w_g, h_g) = grid_screen_dim(); // assumes size 1 font
        let scale = (screen_width() / w_g).max(screen_height() / h_g).ceil();
        let (x, y) = calc_position(scale*w_g, scale*h_g);
        let a = AsciiScreenParams{
            s: s,
            x:x,
            y:y,
            font_size: scale,
        };
        return a;
    }
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
            grid: AsciiGrid::default(),
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
    const RAMP: &[u8] = b" .'`^\\\",;:Iel!><~+_-?][}{)(|/tfjxrnuvcXYZUJCQL0OZmwqpdbhkro*#MW&8%B@$";
    let max_index = RAMP.len() - 1;
    let mut index = ((gray as usize) * max_index) / 255;
    if reverse {
        index = max_index - index;
    }
    RAMP[index] as char
}




// Gets width/height of grid drawn to screen in pixels assuming size one font.
// Use this to determine font size.
fn grid_screen_dim() -> (f32, f32){
    let text_dimensions = measure_multiline_text( // Measure size of macroquad font
        &String::from(&AsciiFrame::default().grid),
        None,
        1,
        1.0,
        Some(1.0));
    return (text_dimensions.width, text_dimensions.height);       
}

