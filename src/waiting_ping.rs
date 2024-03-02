ureq::json!({
    "content":"",
    "embeds": [
        {
            "title": format!("Next Target, {}!", comment),
            "url": format!("https://nationstates.net/region={}", comment),
            "color": 16711680,
            "footer": {
                "text": "Miaou miaou!"
            }
        }
    ],
    "username": "zoomies",
    "attachments": []
  })