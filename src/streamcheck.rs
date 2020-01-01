#![allow(non_snake_case)]
use request::Request;

use std::fmt;
use playlist_decoder;
use url::Url;
use hls_m3u8::MasterPlaylist;
use streamdeepscan;

use log::{debug};

#[derive(Debug)]
pub struct StreamInfo {
    pub Public: Option<bool>,
    pub AudioInfo: String,
    pub Name: String,
    pub Description: String,
    pub Type: String,
    pub Url: String,
    pub Homepage: String,
    pub Genre: String,
    pub Bitrate: u32,
    pub Sampling: u32,
    pub CodecAudio: String,
    pub CodecVideo: Option<String>,
    pub Hls: bool,

    pub LogoUrl: String,
    pub LoadBalancerUrl: String,
    pub IcyVersion: u32,
    pub UseMetaData: bool,
    pub CountryCode: String,
}

#[derive(Debug)]
pub struct StreamCheckError {
    pub Url: String,
    pub Msg: String,
}

impl StreamCheckError {
    fn new(url: &str, msg: &str) -> StreamCheckError {
        StreamCheckError {
            Url: url.to_string(),
            Msg: msg.to_string(),
        }
    }
}

impl fmt::Display for StreamCheckError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.Msg)
    }
}

impl Error for StreamCheckError {
    fn description(&self) -> &str {
        &self.Msg
    }
}

use std::error::Error;
pub type StreamCheckResult = Result<StreamInfo, StreamCheckError>;

fn type_is_m3u(content_type: &str) -> bool {
    return content_type == "application/mpegurl" || content_type == "application/x-mpegurl"
        || content_type == "audio/mpegurl" || content_type == "audio/x-mpegurl"
        || content_type == "application/vnd.apple.mpegurl"
        || content_type == "application/vnd.apple.mpegurl.audio";
}

fn type_is_pls(content_type: &str) -> bool {
    return content_type == "audio/x-scpls" || content_type == "application/x-scpls"
        || content_type == "application/pls+xml";
}

fn type_is_asx(content_type: &str) -> bool {
    return content_type == "video/x-ms-asx" || content_type == "video/x-ms-asf";
}

fn type_is_xspf(content_type: &str) -> bool {
    return content_type == "application/xspf+xml";
}

fn type_is_playlist(content_type: &str) -> bool {
    let search = content_type.find(";");
    let mut content_type = content_type;
    match search {
        Some(index) => {
            content_type = &content_type[0..index];
        }
        None => {}
    }
    return type_is_m3u(content_type) || type_is_pls(content_type) || type_is_asx(content_type)
        || type_is_xspf(content_type);
}

fn type_is_stream(content_type: &str) -> Option<&str> {
    match content_type {
        "audio/mpeg" => Some("MP3"),
        "audio/x-mpeg" => Some("MP3"),
        "audio/mp3" => Some("MP3"),
        "audio/aac" => Some("AAC"),
        "audio/x-aac" => Some("AAC"),
        "audio/aacp" => Some("AAC+"),
        "audio/ogg" => Some("OGG"),
        "application/ogg" => Some("OGG"),
        "video/ogg" => Some("OGG"),
        "audio/flac" => Some("FLAC"),
        "application/flv" => Some("FLV"),
        "application/octet-stream" => Some("UNKNOWN"),
        _ => None,
    }
}

enum LinkType {
    Stream(String),
    Playlist,
    Other
}

fn get_type(content_type: &str, content_length: Option<usize>) -> LinkType {
    let content_type_lower = content_type.to_lowercase();
    if type_is_playlist(&content_type_lower) || content_length.is_some() {
        LinkType::Playlist
    } else if type_is_stream(&content_type_lower).is_some() {
        LinkType::Stream(String::from(type_is_stream(&content_type_lower).unwrap_or("")))
    } else {
        LinkType::Other
    }
}

fn handle_playlist(mut request: Request, url: &str, check_all: bool, timeout: u32, max_depth: u8) -> Vec<StreamCheckResult> {
    let mut list: Vec<StreamCheckResult> = vec![];
    let read_result = request.read_content();
    match read_result {
        Ok(_)=>{
            let content = request.text();
            let is_hls = playlist_decoder::is_content_hls(&content);
            if is_hls {
                let playlist = content.parse::<MasterPlaylist>();
                match playlist{
                    Ok(playlist)=>{
                        for i in playlist.stream_inf_tags() {
                            let mut audio = String::from("UNKNOWN");
                            let mut video: Option<String> = None;
                            let codecs_obj = i.codecs();
                            if let Some(codecs_obj) = codecs_obj {
                                let (a,v) = decode_hls_codecs(&codecs_obj.to_string());
                                audio = a;
                                video = v;
                            }
                            let stream = StreamInfo {
                                Public: None,
                                AudioInfo: String::from(""),
                                Url: String::from(url),
                                Type: String::from(""),
                                Name: String::from(""),
                                Description: String::from(""),
                                Homepage: String::from(""),
                                Bitrate: (i.bandwidth() as u32) / 1000,
                                Genre: String::from(""),
                                Sampling: 0,
                                CodecAudio: audio,
                                CodecVideo: video,
                                Hls: true,
                                LogoUrl: String::from(""),
                                LoadBalancerUrl: String::from(""),
                                IcyVersion: 1,
                                UseMetaData: false,
                                CountryCode: String::from(""),
                            };
                            list.push(Ok(stream));
                            break;
                        }
                    }
                    Err(_)=>{
                        let stream = StreamInfo {
                            Public: None,
                            AudioInfo: String::from(""),
                            Url: String::from(url),
                            Type: String::from(""),
                            Name: String::from(""),
                            Description: String::from(""),
                            Homepage: String::from(""),
                            Bitrate: 0,
                            Genre: String::from(""),
                            Sampling: 0,
                            CodecAudio: String::from("UNKNOWN"),
                            CodecVideo: None,
                            Hls: true,
                            LogoUrl: String::from(""),
                            LoadBalancerUrl: String::from(""),
                            IcyVersion: 1,
                            UseMetaData: false,
                            CountryCode: String::from(""),
                        };
                        list.push(Ok(stream));
                    }
                }
            }else{
                let playlist = decode_playlist(url, &content,check_all, timeout, max_depth - 1);
                if playlist.len() == 0 {
                    list.push(Err(StreamCheckError::new(url, "Empty playlist")));
                } else {
                    list.extend(playlist);
                }
            }
        }
        Err(err)=>{
            list.push(Err(StreamCheckError::new(url, &err.to_string())));
        }
    }
    list
}

fn handle_stream(mut request: Request, url: &str, mut stream_type: String, deep_scan: bool) -> StreamInfo {
    debug!("handle_stream(url={})", url);

    if deep_scan {
        let result = request.read_up_to(50);
        if result.is_ok(){
            let bytes = request.bytes();
            let scan_result = streamdeepscan::scan(bytes);
            if let Ok(scan_result) = scan_result {
                if let Some(scan_result) = scan_result {
                    let x = type_is_stream(&scan_result.mime);
                    if let Some(x) = x {
                        stream_type = String::from(x);
                        debug!("url={}, override stream_type of with deep scan: {}", url, stream_type);
                    }
                }
            }
        }
    }

    let headers = request.info.headers;
    let icy_pub: Option<bool> = match headers.get("icy-pub") {
        Some(content) => {
            let number = content.parse::<u32>();
            match number {
                Ok(number)=>{
                    Some(number == 1)
                },
                Err(_) => {
                    None
                }
            }
        },
        None => {
            None
        }
    };
    let stream = StreamInfo {
        Public: icy_pub,
        AudioInfo: headers.get("icy-audio-info").unwrap_or(&String::from("")).clone(),
        Url: String::from(url),
        Type: headers
            .get("content-type")
            .unwrap_or(&String::from(""))
            .clone(),
        Name: headers.get("icy-name").unwrap_or(&String::from("")).clone(),
        Description: headers
            .get("icy-description")
            .unwrap_or(&String::from(""))
            .clone(),
        Homepage: headers.get("icy-url").unwrap_or(&String::from("")).clone(),
        Bitrate: headers
            .get("icy-br")
            .unwrap_or(&String::from(""))
            .parse()
            .unwrap_or(0),
        Genre: headers
            .get("icy-genre")
            .unwrap_or(&String::from(""))
            .clone(),
        Sampling: headers
            .get("icy-sr")
            .unwrap_or(&String::from(""))
            .parse()
            .unwrap_or(0),
        CodecAudio: stream_type,
        CodecVideo: None,
        Hls: false,
        LogoUrl: headers
            .get("icy-logo")
            .unwrap_or(&String::from(""))
            .clone(),
        LoadBalancerUrl: headers
            .get("icy-loadbalancer")
            .unwrap_or(&String::from(""))
            .clone(),
        IcyVersion: headers
            .get("icy-version")
            .unwrap_or(&String::from(""))
            .parse()
            .unwrap_or(1),
        UseMetaData: headers
            .get("icy-use-metadata")
            .unwrap_or(&String::from("0"))
            .parse()
            .unwrap_or(0) == 1,
        CountryCode: headers
            .get("icy-countrycode")
            .unwrap_or(&String::from(""))
            .clone(),
    };

    stream
}

pub fn check(url: &str, check_all: bool, timeout: u32, max_depth: u8) -> Vec<StreamCheckResult> {
    debug!("check(url={})",url);
    if max_depth == 0{
        return vec![Err(StreamCheckError::new(url, "max depth reached"))];
    }
    let request = Request::new(&url, "StreamCheckBot/0.1.0", timeout);
    let mut list: Vec<StreamCheckResult> = vec![];
    match request {
        Ok(request) => {
            if request.info.code >= 200 && request.info.code < 300 {
                let content_type = request.info.headers.get("content-type");
                let content_length = request.content_length().ok();
                match content_type {
                    Some(content_type) => {
                        let link_type = get_type(content_type, content_length);
                        match link_type {
                            LinkType::Playlist => list.extend(handle_playlist(request, url, check_all, timeout, max_depth)),
                            LinkType::Stream(stream_type) => list.push(Ok(handle_stream(request, url, stream_type, true))),
                            _ => list.push(Err(StreamCheckError::new(url,&format!("unknown content type {}", content_type),)))
                        };
                    }
                    None => {
                        list.push(Err(StreamCheckError::new(
                            url,
                            "Missing content-type in http header",
                        )));
                    }
                }
            } else if request.info.code >= 300 && request.info.code < 400 {
                let location = request.info.headers.get("location");
                match location {
                    Some(location) => {
                        list.extend(check(location,check_all, timeout,max_depth - 1));
                    }
                    None => {}
                }
            } else {
                list.push(Err(StreamCheckError::new(
                    url,
                    &format!("illegal http status code {}", request.info.code),
                )));
            }
        }
        Err(err) => list.push(Err(StreamCheckError::new(url, &err.to_string()))),
    }
    list
}

fn decode_playlist(url_str: &str, content: &str, check_all: bool, timeout: u32, max_depth: u8) -> Vec<StreamCheckResult> {
    let mut list = vec![];
    let base_url = Url::parse(url_str);
    match base_url {
        Ok(base_url) => {
            let urls = playlist_decoder::decode(content);
            let mut max_urls = 10;
            for url in urls {
                max_urls = max_urls - 1;
                if max_urls == 0{
                    break;
                }
                if url.trim() != "" {
                    let abs_url = base_url.join(&url);
                    match abs_url {
                        Ok(abs_url) => {
                            let result = check(&abs_url.as_str(),check_all, timeout, max_depth);
                            if !check_all{
                                let mut found = false;
                                for result_single in result.iter() {
                                    if result_single.is_ok() {
                                        found = true;
                                    }
                                }
                                if found{
                                    list.extend(result);
                                    break;
                                }
                            }
                            list.extend(result);
                        }
                        Err(err) => {
                            list.push(Err(StreamCheckError::new(
                                url_str,
                                &err.to_string(),
                            )));
                        }
                    }
                }
            }
        }
        Err(err) => {
            list.push(Err(StreamCheckError::new(
                url_str,
                &err.to_string(),
            )));
        }
    }

    list
}

fn decode_hls_codecs(codecs_raw: &str) -> (String,Option<String>) {
    // codec information from
    // https://developer.apple.com/library/content/documentation/NetworkingInternet/Conceptual/StreamingMediaGuide/FrequentlyAskedQuestions/FrequentlyAskedQuestions.html

    let mut codec_audio: String = String::from("UNKNOWN");
    let mut codec_video: Option<String> = None;

    if codecs_raw.contains("mp4a.40.2") {
        // AAC-LC
        codec_audio = String::from("AAC");
    }
    if codecs_raw.contains("mp4a.40.5") {
        // HE-AAC
        codec_audio = String::from("AAC+");
    }
    if codecs_raw.contains("mp4a.40.34") {
        codec_audio = String::from("MP3");
    }
    if codecs_raw.contains("avc1.42001e") || codecs_raw.contains("avc1.66.30") {
        // H.264 Baseline Profile level 3.0
        codec_video = Some(String::from("H.264"));
    }
    if codecs_raw.contains("avc1.42001f") {
        // H.264 Baseline Profile level 3.1
        codec_video = Some(String::from("H.264"));
    }
    if codecs_raw.contains("avc1.4d001e") || codecs_raw.contains("avc1.77.30") {
        // H.264 Main Profile level 3.0
        codec_video = Some(String::from("H.264"));
    }
    if codecs_raw.contains("avc1.4d001f") {
        // H.264 Main Profile level 3.1
        codec_video = Some(String::from("H.264"));
    }
    if codecs_raw.contains("avc1.4d0028") {
        // H.264 Main Profile level 4.0
        codec_video = Some(String::from("H.264"));
    }
    if codecs_raw.contains("avc1.64001f") {
        // H.264 High Profile level 3.1
        codec_video = Some(String::from("H.264"));
    }
    if codecs_raw.contains("avc1.640028") {
        // H.264 High Profile level 4.0
        codec_video = Some(String::from("H.264"));
    }
    if codecs_raw.contains("avc1.640029") {
        // H.264 High Profile level 4.1
        codec_video = Some(String::from("H.264"));
    }

    return (codec_audio,codec_video);
}