ureq::json!({
    "content":"",
    "embeds": [
        {
            "title": format!("{}", comment),
            "url": format!("https://nationstates.net/region={}", target),
            "color": 16711680,
            "footer": {
                "text": "Miaou miaou!"
            }
        }
    ],
    "username": "zoomies",
    "attachments": []
  })