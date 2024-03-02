ureq::json!({
    "content":"",
    "embeds": [
        {
            "title": "zoomies time, miaou miaou!",
            "url": "https://www.youtube.com/watch?v=ImXqPF7iqDU",
            "description": format!("`zoomies` v{}\n{} is going {}miaou/ms!", env!("CARGO_PKG_VERSION"), main_nation, poll_speed),
            "color": 16711680,
            "image": {
                "url": "https://github.com/Vleerian/Zoomies/blob/main/assets/Cat.png?raw=true"
            },
            "footer": {
                "text": "miaou miaou!"
            }
        }
    ],
    "username": "zoomies",
    "avatar_url": "https://github.com/Vleerian/Zoomies/blob/main/assets/Cat.png?raw=true",
    "attachments": []
  })