ureq::json!({
  "content":"",
  "embeds": [
      {
          "title": format!("{}", update_message),
          "color": 16711680,
          "footer": {
              "text": format!("{}", timestring)
          }
      }
  ],
  "username": "zoomies",
  "attachments": []
})