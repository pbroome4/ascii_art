use image::codecs::gif::{GifDecoder};
use image::imageops::FilterType::{self, Gaussian, Nearest};
use image::imageops::{grayscale, resize};
use image::metadata::LoopCount::{self, Finite};
use image::{AnimationDecoder, Frame, GenericImageView, ImageBuffer, Pixel, imageops};
use macroquad::rand::{RandomRange};
use std::{io};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek};
use std::path::{PathBuf};
use macroquad::{prelude::*};
use std::ops::{Index, IndexMut};
use std::time::{Duration, Instant};
use std::fmt::Display;

use crate::DisplayStates::Blur;


const ASPECT_RATIO: f32 = 9.0 / 9.0;  // width px / height px
const FONT_ASPECT_RATIO: f32 = 0.4;//3.0 / 5.0; // width char / height char 
const ASCII_COLS: usize = 500;  // width in chars
const ASCII_ROWS: usize = (ASCII_COLS as f32  * FONT_ASPECT_RATIO / ASPECT_RATIO) as usize;
const LOOP_DUR: Duration = Duration::from_secs(2); // If a gif is a looper, loop for this duration
const TIME_DIALATE_FACTOR:f32 = 2.0;

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

    rand::srand(miniquad::date::now() as u64);

    let blur_dur = Duration::from_secs(2); // seconds
    let focus_duration = Duration::from_secs(2); // seconds
    let mut curr_gif = &gifs[0];
    let mut curr_grid = AsciiGrid::default();
    let mut curr_state = DisplayStates::Blank;
    let mut last_state_update = Instant::now();

    let mut blur_grid_start = AsciiGrid::default();
    let mut blur_grid_end = AsciiGrid::default();

    let mut focus_grid_start = AsciiGrid::default();
    let mut focus_grid_end = AsciiGrid::default();
    
    let mut play_frames = curr_gif.iter();
    let mut play_frame = play_frames.next().unwrap();
    let mut last_play_update = Instant::now();

    loop{
        match curr_state{
            DisplayStates::Blank =>{
                println!("State Blank - >Blur");
                curr_state = DisplayStates::Blur;
                curr_grid = AsciiGrid::default();
                blur_grid_start = curr_grid.clone();
                blur_grid_end = AsciiGrid::random();
            }
            DisplayStates::Blur => {
                let mut i = (Instant::now() - last_state_update).as_secs_f64() / blur_dur.as_secs_f64();
                i = i.min(1.0);
                curr_grid = blur_grid_start.add_eps(&blur_grid_end, i as f32);
                
                if i >= 1.0 {
                    println!("State Blur -> Focus");
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
                if i >= 1.0{
                    println!("State Focus -> Play");
                    curr_state = DisplayStates::Play;
                    play_frames = curr_gif.iter();
                    last_play_update = Instant::now();
                    last_state_update =Instant::now();
                }
            }

            DisplayStates::Play => {                
                if Instant::now() - last_play_update > play_frame.delay.mul_f32(TIME_DIALATE_FACTOR){
                    if let Some(next_play_frame) = play_frames.next(){
                        play_frame = next_play_frame;
                        curr_grid = play_frame.grid.clone();
                        last_play_update = Instant::now();
                    }else{
                        println!("State Play -> Blur");
                        curr_state = Blur;
                        blur_grid_start = curr_grid.clone();
                        blur_grid_end = AsciiGrid::random();
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

#[derive(Clone, Debug)]
struct AsciiGrid{
    g: Vec<Vec<u8>> //col x row
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
        Self { g: vec![vec![0;ASCII_ROWS]; ASCII_COLS] }
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
    type Output = Vec<u8>;
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

#[derive(Debug)]
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

impl From<Frame> for AsciiFrame{
    fn from(frame: Frame) -> Self {
        let mut ascii = AsciiFrame::default();
        ascii.delay = frame.delay().into();
        let im_buf = frame.into_buffer();
        let im_buf = stretch_hw( // predistort the image bc font not square
            &im_buf,
            FONT_ASPECT_RATIO,
            Gaussian);
        let im_buf = resize_to_fill( 
            &im_buf,
            ASCII_COLS as u32,
            ASCII_ROWS as u32,
            true,
            Gaussian);
        let im_buf = crop_center(
            &im_buf,
            ASCII_COLS as u32,
            ASCII_ROWS as u32
        ); 
        let im_gray_scale = grayscale(&im_buf);
        for (x, y, p) in im_gray_scale.enumerate_pixels(){
            ascii.grid[x as usize][y as usize] = p[0];
        }
        //println!("Created AsciiFrame: {:?}", ascii);
        return ascii;
    }
}


#[derive(Debug)]
struct AsciiGif{
    frames: Vec<AsciiFrame>,
    loop_count: usize,
}

impl AsciiGif{
    fn iter(&self) -> AsciiGifIterator{
        let i = AsciiGifIterator{
            a: self,
            it_index: 0,
        };
        return i;
    }

    fn calc_duration(&self) -> Duration{
        let mut d = Duration::from_secs(0);
        for frame in self.frames.iter(){
            d += frame.delay
        }
        return d;
    }
}

impl <R> From<GifDecoder<R>> for AsciiGif
where 
    R: BufRead + Seek
{
    // 
    fn from(decoder: GifDecoder<R>) -> Self {
        let lc = decoder.loop_count();    
        let frames = decoder.into_frames()
                            .collect_frames()
                            .expect("error decoding gif");
        let loop_count = calc_loop_count(&frames, &lc, LOOP_DUR);
        let mut v = Vec::new();
        for frame in frames{
            let a = AsciiFrame::from(frame);
            v.push(a);
        }
        let s = Self {
            frames: v,
            loop_count: loop_count,
        };
        println!("Created AsciiGif: {}", &s);
        return s;
    }
}

impl Display for AsciiGif{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "AsciiGif is frames.len(): {}, duration: {}, loop_count: {}}}",
        self.frames.len(), self.calc_duration().as_secs_f32(), self.loop_count)
    }
}


impl From<PathBuf> for AsciiGif{
    fn from(value: PathBuf) -> Self {
        let file_in = BufReader::new(File::open(value).unwrap());
        let decoder = GifDecoder::new(file_in).unwrap();
        return AsciiGif::from(decoder);
    }
}


struct AsciiGifIterator<'a>{
    a: &'a AsciiGif,
    it_index:usize
}

impl <'a> Iterator for AsciiGifIterator<'a>{
    type Item = &'a AsciiFrame;
    
    fn next(&mut self) -> Option<Self::Item> {
            if self.it_index > self.a.frames.len() * self.a.loop_count{
                return None;
            }
            let r = &self.a.frames[self.it_index % self.a.frames.len()];
            self.it_index += 1;
            return Some(r);
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

fn calc_duration(v: &Vec<Frame>)-> Duration{
    let mut d = Duration::from_secs(0);
    for f in v.iter(){
        d += f.delay().into();
    }
    return d;
}

// If loopcount is 1, treat gif as non repeating and return 1
// If loop count is >1 or infinite, treat it as repeating and return 
// #loops such that gif is at least target_dur long.
fn calc_loop_count(v: &Vec<Frame>, lc: &LoopCount, target_dur: Duration) -> usize{
    let loop_count: usize = match lc{
        Finite(n) => {
            let mut l = 1;
            if n.get() > 1{
                println!("Loop count > 1");
                let d = calc_duration(v);
                l = (target_dur.as_secs_f32() / d.as_secs_f32()).ceil() as usize
            }
            l
        }
        LoopCount::Infinite => {
            println!("Loop count: infinite");
            let d = calc_duration(v);
            (target_dur.as_secs_f32() / d.as_secs_f32()).ceil() as usize
        }
    };
    return loop_count;
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
    let mut im_buf = imageops::resize(i, i.width(), i.height(), Gaussian); // TODO how else to convert image to imagebuf?
    if w > im_buf.width(){
        println!("Warning, crop width larger than image width, resizing image instead");
        im_buf = resize(&im_buf, w, im_buf.height(), Gaussian);
    }
    if h > im_buf.height(){
        println!("Warning, crop height is larger than image height, resizing image instead");
        im_buf = resize(&im_buf, im_buf.width(), h, Gaussian);
    }
    let x0 = im_buf.width()/2 - w/2;
    let y0 = im_buf.height()/2 - h/2;
    let im_buf  = imageops::crop_imm(&im_buf, x0, y0, w, h).to_image();
    return im_buf;
}


// Converts grayscale value to ascii char
// from value from 0 (empty) to 255 (Filled)
fn get_ascii_char(gray: u8, reverse: bool) -> char {
    //const RAMP: &[u8] = b" .'`^\",;:lI!i<>+~_-?[]{}()|\\/tfjrxnuvczyXUJLCQ0OZmwqpdbhkhao*#MW&8%B@$";
    const RAMP: &[u8] = b" .:-=+*#%@";
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

