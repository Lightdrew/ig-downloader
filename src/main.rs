
async fn fetch_reel(url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    
    let mut video_downloaded = false;

    let response = reqwest::Client::new()
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.5")
        .header("Sec-GPC", "1")
        .header("Alt-Used", "www.instagram.com")
        .header("Cache-Control", "no-cache")
        .header("Pragma", "no-cache")
        .header("Upgrade-Insecure-Requests", "1")
        .header("Sec-Fetch-Dest", "document")
        .header("Sec-Fetch-Mode", "navigate")
        .header("Sec-Fetch-Site", "none")
        .header("Sec-Fetch-User", "?1")
        .header("Priority", "u=0, i")
        .send()
        .await?;
    
    if response.status().is_success() {
        println!("Response: {}", response.status());

        if let Ok(text) = response.text().await {
            let document = tl::parse(&text, tl::ParserOptions::default()).unwrap();
            let parser = document.parser();

            if let Some(mut scripts) = document.query_selector("script[type=\"application/json\"]")
            {
                loop {
                    
                    let script = match scripts.next() {
                        Some(script) => script,
                        None => break,
                    };
                    
                    let script_text = match script.get(parser) {
                        Some(script_text) => script_text.inner_html(parser),
                        None => continue,
                    };

                    let json_script = match serde_json::from_str(&script_text)? {
                        serde_json::Value::Object(json_script) => json_script,
                        _ => continue,
                    };

                    // Commence Json navigation hell

                    let video = json_script.get("require")
                        .and_then(|x| x.get(0))
                        .and_then(|x| x.get(3))
                        .and_then(|x| x.get(0))
                        .and_then(|x| x.get("__bbox"))
                        .and_then(|x| x.get("require"))
                        .and_then(|x| x.get(0))
                        .and_then(|x| x.get(3))
                        .and_then(|x| x.get(1))
                        .and_then(|x| x.get("__bbox"))
                        .and_then(|x| x.get("result"))
                        .and_then(|x| x.get("data"));

                    let video = match video {
                        Some(video) => 
                        {
                            video.get("xdt_api__v1__media__shortcode__web_info")
                            .or_else(|| video.get("xdt_api__v1__clips__home__connection_v2"))
                            .or_else(|| video.get("xdt_api__v1__clips__home__no__login__connection_v2"))
                        },
                        None => continue,
                    };
                    
                    let video = match video {
                        Some(video) => 
                        {
                            video
                            .get("edges")
                            .or_else(|| video.get("items"))
                        },
                        None => continue,
                    };

                    let video = match video {
                        Some(video) => video.get(0),
                        None => continue,
                    };

                    match video {
                        Some(video) => {

                            //Attempt downloading the video

                            let code = video["code"].as_str().unwrap_or("unknown");
                            let author = video["user"]["username"].as_str().unwrap_or("unknown");
                            
                            println!("Code: {}", code);
                            println!("Author: {}", author);
                            
                            let url = video["video_versions"][0]["url"].to_string().replace("\"", "");

                            println!("Downloading video...");
                            
                            let video_response = reqwest::get(&url).await?;

                            if video_response.status().is_success() 
                            {
                                let video_bytes = video_response.bytes().await?;

                                tokio::fs::create_dir_all("download").await?;

                                let mut file = tokio::fs::File::create(format!("download/{}-{}.mp4",author,code)).await?;
                                
                                tokio::io::copy(&mut video_bytes.as_ref(), &mut file).await?;
                                
                                println!("Video downloaded successfully as {}-{}.mp4",author,code);

                                video_downloaded = true;
                            } else 
                            {
                                return Err(Box::new(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    format!("Failed to download video: Request failed ({})", video_response.status()),
                                )));
                            }
                        }
                        None => {
                            continue
                        },
                    }
                }
            }
            else {

                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to fetch reel: No data scripts found",
                )));
            }


        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to fetch reel: Failed to parse response",
            )));
        }


    }
    else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to fetch reel: Request failed ({})", response.status()),
        )));
    }

    match video_downloaded {
        true => Ok(()),
        false => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to fetch reel: No video found",
        ))),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("Usage: {} <instagram_url>", args[0]);
        std::process::exit(1);
    }

    let url: &String = &args[1].clone().replace("reels", "reel");

    let mut retries = 5;

    let mut result = fetch_reel(url).await;

    while result.is_err() && retries > 1 {
        println!("Error: {:?}", result.err());
        println!("Retrying...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        result = fetch_reel(url).await;
        retries -= 1;
    }


    Ok(())
}
