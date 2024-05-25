use std::fs::File;
use std::io::{self, Cursor, Read, Write};
use std::path::Path;
use image::io::Reader as ImageReader;
use image::DynamicImage;
use rouille::{post_input, router, try_or_400};
use zip::write::SimpleFileOptions;

static FORM: &'static str = r###"
<!doctype html>
<head>
    <title>Tyler</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.min.css"/>
</head>

<body>
    <main class="container">
        <h1>Tyler 🟦</h1>
        <div>
            <p>Upload an image and choose a tile size (in pixels) to split your image into tiles. </p>

            <p>Tyler will generate a zip file of images, named <code>tile_{col-num}_{row-num}.png</code>.
        </div>
        
        <div>
            <form action="submit" method="POST" enctype="multipart/form-data">
                <img id="preview" src="#" alt=""  style="max-width: 100%" />
                <p><input id="imgSelect" type="file" name="files" multiple /></p>
                <p>Tile Size (px): <input type="number" name="width" value="128" step="1", min="8" /></p>
                <p><button>Tile!</button></p>
            </form>
        </div>
        <script>
            imgSelect.onchange = evt => {
                const [file] = imgSelect.files
                if (file) {
                    preview.src = URL.createObjectURL(file)
                }
            }
        </script>
    </main>
</body>

"###;

fn main() {
    http_server(8080);
}

fn http_server(port: u16){

    rouille::start_server_with_pool(format!("0.0.0.0:{}", port), Some(8), move |request| {
        rouille::log(&request, io::stdout(), || {
            router!(request,
                (GET) (/) => {
                    rouille::Response::html(FORM)
                },
                (POST) (/submit) => {
                    let data = try_or_400!(post_input!(request, {
                        width: u32,
                        files: Vec<rouille::input::post::BufferedFile>,
                    }));

                    println!("{}", data.width);

                    let image = &data.files.get(0).unwrap().data;

                    let tiles = tile_image(image.to_vec(), data.width).unwrap();

                    let mut buf: Vec<u8> = Vec::new(); // Declare buf as mutable

                    write_tile_zip(&mut buf, tiles).unwrap();

                    let resp = rouille::Response::from_data("application/x-zip", buf).with_content_disposition_attachment("tiles.zip");

                    resp

                },
                _ => rouille::Response::empty_404()
            )
        })
    });
}

// fn write_tile_zip_path(path: &Path, tiles: Vec<Vec<DynamicImage>>) -> Result<(), Box<dyn std::error::Error>> {
//     let file = File::create(path)?;

//     return write_tile_zip(file, tiles);
// }

fn write_tile_zip(buf: &mut Vec<u8>, tiles: Vec<Vec<Vec<u8>>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(buf));

    for (y, row) in tiles.iter().enumerate() {
        for (x, tile) in row.iter().enumerate() {

            // We now have a byte vector, `bytes`, containing the tile. We need to write it to the zip.

            zip.start_file(format!("tile_{}_{}.png", y, x), SimpleFileOptions::default())?;
            zip.write_all(tile)?;
        }
    }

    zip.finish()?;

    Ok(())
}

fn tile_image_path(path: &Path, tile_size: u32) -> Result<Vec<Vec<Vec<u8>>>, Box<dyn std::error::Error>>{
    let mut buf: Vec<u8> = Vec::new();

    File::open(path)?.read(&mut buf)?;

    tile_image(buf, tile_size)

}


fn tile_image(bytes: Vec<u8>, tile_size: u32) -> Result<Vec<Vec<Vec<u8>>>, Box<dyn std::error::Error>> {
    let img = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?.decode()?;
    
    let mut tiles = Vec::new();
    
    let num_rows = (img.height() as f32 / tile_size as f32).ceil() as u32;
    let num_cols = (img.width() as f32 / tile_size as f32).ceil() as u32;
    
    for y in 0..num_rows {
        let mut row = Vec::new();
        for x in 0..num_cols {
            let tile = img.crop_imm(x * tile_size, y * tile_size, tile_size, tile_size);
            let mut tile_bytes = Vec::new();
            tile.write_to(&mut Cursor::new(&mut tile_bytes), image::ImageFormat::Png)?;
            row.push(tile_bytes);
        }
        tiles.push(row);
    }
    
    Ok(tiles)
}