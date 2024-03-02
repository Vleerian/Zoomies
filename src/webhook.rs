ureq::json!({
  "content":"",
  "embeds": [
      {
          "title": format!("{}", update_message),
          "url": format!("https://nationstates.net/region={}", trigger.region),
          "color": 16711680,
          "footer": {
              "text": format!("{}", timestring)
          }
      }
  ],
  "username": "zoomies",
  "attachments": []
})