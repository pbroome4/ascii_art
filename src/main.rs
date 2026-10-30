use image::ImageFormat::Png;
use image::codecs::gif::{GifDecoder};
use image::imageops::FilterType::Nearest;
use image::imageops::grayscale;
use image::{AnimationDecoder, Frame, Frames, ImageDecoder, Pixel, imageops};
use macroquad::miniquad::date::now;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::vec;
use macroquad::prelude::*;


// Keep 9:16 aspect ratio.
const ASCII_ROWS: usize = 40;
const ASCII_COLS: usize = 40;
type  AsciiGrid = [[char;ASCII_ROWS];ASCII_COLS];

//////////////////////////////////////////////////////
//  Parse resources folder
//      For each resource:
//          parse gif bits
//          convert to fixed resolution + aspect ratio
//          convert to ascii chars 
//
//  
//



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
        let width = screen_width();
        let height = screen_height();
        let font_size = height / ASCII_ROWS as f32
        ;
        let frame = &gif.frames[frame_n % gif.frames.len()];
        clear_background(BLACK);
        let s = grid_to_string(&frame.grid);
        draw_multiline_text(s, 20.0, 20.0, font_size, Some(1.0), WHITE);
        if get_time() > next_update{
            frame_n += 1;
            next_update = get_time() + frame.delay.as_secs_f64();   
        }
    
        next_frame().await
    }
}

enum ArtState {
    Scrambling
}

struct AsciiFrame{
    grid: AsciiGrid,
    delay: Duration,  // seconds
}

impl Default for AsciiFrame{
    fn default() -> Self {
        Self {
            grid: [[0 as char; ASCII_ROWS]; ASCII_COLS],
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
            let im_buf = imageops::resize(&im_buf,
                                          ASCII_COLS as u32,
                                          ASCII_ROWS as u32,
                                          Nearest);
            let im_gray_scale = grayscale(&im_buf);
            for (x, y, p) in im_gray_scale.enumerate_pixels(){
                ascii.grid[x as usize][y as usize] = get_ascii_char(p[0], false);
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
            println!("Detected GIF resource: {}", path_str);
            let a = AsciiGif::from(path);
            v.push(a);
        }
    }
    return v;
}

fn read_ascii_image()->Vec<Vec<char>>{
    // Decode a gif into frames
    let file_in = BufReader::new(File::open("resources/fire.gif").unwrap());
    let mut decoder = GifDecoder::new(file_in).unwrap();
    let frames = decoder.into_frames();
    let frames = frames.collect_frames().expect("error decoding gif");
    let im_buf = frames.into_iter().next().unwrap().into_buffer();
    im_buf.save_with_format("fire_out", image::ImageFormat::Png).unwrap();
    let im_buf2 = imageops::resize(&im_buf, 40, 40, Nearest);
    let im_gray_scale = grayscale(&im_buf2);
    let mut ascii = vec![vec!['a'; im_buf2.width() as usize]; im_buf2.height() as usize];
    for (x, y, p) in im_gray_scale.enumerate_pixels(){
        ascii[x as usize][y as usize] = get_ascii_char(p[0], false);
    }
    
    return ascii;
}


// Converts grayscale value to ascii char
// from value from 0 (empty) to 255 (Filled)
fn get_ascii_char(gray: u8, reverse: bool) -> char {
    const RAMP: &[u8] = b" .'`^\\\",;:Iel!><~+_-?][}{)(|/tfjxrnuvcXYZUJCQL0OZmwqpdbhkro*#MW&8%B@$";
    //const RAMP: &[u8] = b" .-=*#@";
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
            s.push(grid[c][r]);
        }
        s.push('\n');
    }

    return s;
}