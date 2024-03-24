use maud::{html, Markup, DOCTYPE};

#[get("/")]
pub fn root() -> Markup {
    html!(
        (DOCTYPE)

        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";

                meta property="og:title" content="VRChat YouTube Proxy" name="title";
                meta property="og:description" content="Play YouTube videos on Quest in VRChat!" name="description";
                meta property="og:type" content="website";
                meta property="og:url" content="https://shay.loan";
                meta property="og:image" content="https://socialify.git.ci/ShayBox/VRC-YT/png";
                meta property="og:width" content="1280";
                meta property="og:height" content="640";

                title { "VRChat YouTube Proxy" }

                link rel="shortcut icon" href="https://www.youtube.com/s/desktop/0f05c280/img/favicon.ico" type="image/x-icon";
                link rel="icon" href="https://www.youtube.com/s/desktop/0f05c280/img/favicon_32x32.png" sizes="32x32";
                link rel="icon" href="https://www.youtube.com/s/desktop/0f05c280/img/favicon_48x48.png" sizes="48x48";
                link rel="icon" href="https://www.youtube.com/s/desktop/0f05c280/img/favicon_96x96.png" sizes="96x96";
                link rel="icon" href="https://www.youtube.com/s/desktop/0f05c280/img/favicon_144x144.png" sizes="144x144";
                link rel="stylesheet" href="https://fonts.googleapis.com/css?family=Dosis";

                style {"
                    body {
                        background-color: #111111;
                        color: white;
                        font-family: 'Dosis', ui-rounded;
                        font-size: x-large;
                    }
            
                    div {
                        align-items: center;
                        display: flex;
                        flex-direction: column;
                        height: 98vh;
                        /* Hide Scrollbar */
                        justify-content: center;
                    }
            
                    a {
                        color: inherit;
                    }
            
                    a:hover {
                        color: deepskyblue;
                    }
                "}
            }

            body {
                div {
                    div {
                        h1 { "VRChat YouTube Proxy" }
                        h2 { "Play YouTube videos on Quest in VRChat!" }
                    }

                    div {
                        h3 style="color:red" {
                            span style="color:gray" { "https://" }
                            "www.youtube.com/watch?v="
                            span style="color:deepskyblue" { "dQw4w9WgXcQ" }
                        }

                        h3 style="color:lime" {
                            span style="color:gray" { "https://" }
                            "shay.loan/"
                            span style="color:deepskyblue" { "dQw4w9WgXcQ" }
                        }
                    }

                    footer {
                        a href="https://github.com/ShayBox/VRC-YT" { "Source code" }
                    }
                }
            }
        }
    )
}
