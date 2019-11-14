extern crate multipart;
extern crate iron;
extern crate time;
//image converter
extern crate image;
extern crate crypto;
extern crate rustc_serialize;
use rustc_serialize::json;

use crypto::md5::Md5;
use crypto::digest::Digest;

use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use multipart::server::{Multipart, Entries, SaveResult};

use iron::prelude::*;
use iron::status;
use iron::mime::Mime;

extern crate router;
use router::Router;

const INDEX_HTML: &'static [u8] = include_bytes!("../index.html");
const DIR: &str = "imgs";

#[derive(Debug, RustcDecodable, RustcEncodable)]
struct JsonResult  {
    ret: bool,
    data: String,
}

fn main() {
    let mut router = Router::new();

    router.get("/", |_: &mut Request| {
        let content_type = "text/html".parse::<Mime>().unwrap();
        Ok(Response::with((content_type, status::Ok, INDEX_HTML)))
    },"index");
    router.post("upload", process_upload,"upload");
    
    router.get("/:md5", process_query,"md5");
    router.get("/:md5/:ext", process_query,"md5ext");

    router.get("error", |_: &mut Request| {
      Ok(Response::with(status::BadRequest))
    },"error");

    Iron::new(router).http("0.0.0.0:8080").unwrap();

    // Iron::new(process_upload).http("localhost:8080").expect("Could not bind localhost:8080");
}

///process query
fn process_query(request: &mut Request) -> IronResult<Response> {
   let ref md5 = request.extensions.get::<Router>().unwrap().find("md5").unwrap_or("/");
   let mut ftype = request.extensions.get::<Router>().unwrap().find("ext").unwrap_or("/");
   let mut content_type = "image/jpeg".parse::<Mime>().unwrap();
   let mut ext = image::JPEG;
   if ftype == "png" {
       content_type = "image/png".parse::<Mime>().unwrap();
       ext = image::PNG;
   }else {
       ftype = "jpg";
   }
   let dp1 = &md5[..3];
   let dp2 = &md5[3..6];
   let img = match image::open(format!("{}/{}/{}/{}.{}",DIR,dp1,dp2,md5,ftype)) {
       Ok(img) => img,
       Err(e) => return Err(IronError::new(e, status::InternalServerError))
   };

   // let thumb = img.resize(128, 128, image::FilterType::Triangle);
   let mut buffer = vec![];

   match img.save(&mut buffer, ext) {
       Ok(_) => Ok(Response::with((content_type,iron::status::Ok, buffer))),
       Err(e) => Err(IronError::new(e, status::InternalServerError))
   }
}

/// Processes a request and returns response or an occured error.
fn process_upload(request: &mut Request) -> IronResult<Response> {
    // Getting a multipart reader wrapper
        match Multipart::from_request(request) {
            Ok(mut multipart) => {
                // Fetching all data and processing it.
                // save_all() reads the request fully, parsing all fields and saving all files
                // in a new temporary directory under the OS temporary directory.
                match multipart.save_all() {
                    SaveResult::Full(entries) => process_entries(entries),
                    SaveResult::Partial(entries, error) => {
                        (process_entries(entries))?;
                        Err(IronError::new(error, status::InternalServerError))
                    }
                    SaveResult::Error(error) => Err(IronError::new(error, status::InternalServerError)),
                }
            }
            Err(_) => {
                Ok(Response::with((status::BadRequest, "The request is not multipart")))
            }
        }
}

/// Processes saved entries from multipart request.
/// Returns an OK response or an error.
fn process_entries(entries: Entries) -> IronResult<Response> {
    let mut md5s = String::new();
    for (name, field) in entries.fields {
        println!(r#"Field "{}": "{}""#, name, field);
    }

    for (name, savedfile) in entries.files {
        let filename = match savedfile.filename {
            Some(s) => s,
            None => "None".into(),
        };
        let file_start = time::now();
        let mut file = match File::open(savedfile.path) {
            Ok(file) => file,
            Err(error) => {
                return Err(IronError::new(error,
                                          (status::InternalServerError,
                                           "Server couldn't save file")))
            }
        };
        let file_end = time::now();//
        println!("file load!start : {},end :{},duration:{}",file_start.rfc3339(),file_end.rfc3339(),file_end-file_start);
        //caculate md5
        let mut buffer = Vec::new();
        // read the whole file
        file.read_to_end(&mut buffer).unwrap();
        let mut hasher = Md5::new();
        hasher.input(&buffer);
        let md5 = hasher.result_str();
        // println!("{}", md5);
        md5s = md5s + &md5 + ",";
        let md5_end = time::now();//
        println!("md5 load!start : {},end :{},duration:{}",file_end.rfc3339(),md5_end.rfc3339(),md5_end-file_end);
        //image file
        let img = match image::load_from_memory(&buffer){
            Ok(file) => file,
            Err(error) => {
                return Err(IronError::new(error,
                                          (status::InternalServerError,
                                           "Unsupported image format")))
            }
        };
        let img_end = time::now();//
        println!("img load!start : {},end :{},duration:{}",md5_end.rfc3339(),img_end.rfc3339(),img_end-md5_end);
        let dp1 = &md5[..3];
        let dp2 = &md5[3..6];
        // println!("{:?},{:?}",dp1,dp2 );
        // println!("`mkdir -p a/c/d`");
           // 递归地创建目录，返回 `io::Result<()>`
           fs::create_dir_all(format!("{}/{}/{}",DIR,dp1,dp2)).unwrap_or_else(|why| {
               println!("! {:?}", why.kind());
           });
        let ref mut fout = File::create(&Path::new(&*(format!("{}/{}/{}/{}.jpg",DIR,dp1,dp2,md5)))).unwrap();
        let ref mut fpout = File::create(&Path::new(&*(format!("{}/{}/{}/{}.png",DIR,dp1,dp2,md5)))).unwrap();
        // The dimensions method returns the images width and height
        // println!("dimensions {:?}", img.dimensions());
        // The color method returns the image's ColorType
        // println!("{:?}", img.color());

        // Write the contents of this image to the Writer in PNG format.
        let _ = img.save(fout, image::JPEG).unwrap();
        let _ = img.save(fpout, image::PNG).unwrap();

        let save_end = time::now();//
        println!("save file!start : {},end :{},duration:{}",img_end.rfc3339(),save_end.rfc3339(),save_end-img_end);

        println!(r#"Field "{}" is file "{}":"#, name, filename);
    }
    let content_type = "application/json".parse::<Mime>().unwrap();
    let object = JsonResult{
        ret:true,
        data:md5s,
    };
    Ok(Response::with((content_type, status::Ok, json::encode(&object).unwrap())))
    // Ok(Response::with((status::Ok, md5s)))
}