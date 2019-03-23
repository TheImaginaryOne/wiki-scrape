use kuchiki::traits::*;
use std::borrow::Borrow;
use unicode_segmentation::UnicodeSegmentation;

// fn run() -> Result<(), Box<std::error::Error>> {
//     println!("Scraping...");

//     let mut resp = reqwest::get("https://en.wikipedia.org/api/rest_v1/page/html/Philosophy")?;
//     let resp_html = resp.text()?;
//     //let mut resp_html = r"<html><body>nihaoma?</body></html>";
//     let doc = kuchiki::parse_html().one(resp_html);

//     for node_ref in doc.select("section p a[rel~='mw:WikiLink']").unwrap() {
//         let node = node_ref.as_node();

//         // millions of unwrap() and borrow()
//         let node_el = node.as_element().unwrap().borrow();
//         let link_title = node_el.attributes.borrow().get("href").borrow().unwrap().to_string();
//         println!("link: {}", link_title);
//     }

//     // let mut text = text_node.as_text().unwrap().borrow().to_string();
//     // text.truncate(5000);
    
//     Ok(())
// }

/// Deletes things contained in parentheses
/// as the first wikilink we want must be "in the main text", meaning not parenthesised.
/// We keep parentheses inside tag attributes however.
/// (hello)hello changes to hello, but <a href="hello_(hello)"></a> is unchanged.
fn delete_parentheses<S: Into<String>>(input_str: S) -> String {
    let mut parenth_level = 0;
    let mut tag_level = 0;
    let mut result: String = "".to_string();
    let s: String = input_str.into();
    for c in s.graphemes(true) {
        if parenth_level <= 0 {
            if c == "<" {
                tag_level += 1;
            }
            if c == ">" {
                tag_level -= 1;
            }
        }
        if tag_level <= 0 {
            if c == "(" {
                parenth_level += 1;
            }
            // note: this if clause must be first otherwise there will be dangling )
            if parenth_level == 0 {
                result += c;
            }
            if c == ")" {
                parenth_level -= 1;
            }
        }
        else {
            result += c;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::delete_parentheses;
    #[test]
    fn parenth1() {
        assert_eq!(delete_parentheses("hello (hello (hello) hello) hello"), "hello  hello")
    }
    #[test]
    fn parenth2() {
        assert_eq!(delete_parentheses("()"), "")
    }
    #[test]
    fn parenth3() {
        assert_eq!(delete_parentheses("<a href='hello_(hello)'>(hello)hello</a>"), "<a href='hello_(hello)'>hello</a>")
    }
    #[test]
    fn parenth4() {
        assert_eq!(delete_parentheses("<a>(<b>)<b>(f<a>jjj</a>>)hello"), "<a><b>hello")
    }
}

fn follow_first_links(initial_page: &str, final_page: &str) -> Result<(), Box<std::error::Error>> {
    let client = reqwest::Client::new();

    println!(">> Following first wikilinks of each page");
    let mut count = 0;
    let mut current = format!("{}", initial_page);
    // the rest_v1 html has lots additional metadata
    let base_url = url::Url::parse("https://en.wikipedia.org/api/rest_v1/page/html/").unwrap();

    loop {
        if current == final_page {
            println!("Reached {} in {} clicks!", final_page, count);
            return Ok(());
        }
        println!("Visiting {}...", current);
        let url = base_url.join(&current).unwrap();
        let mut resp = client.get(url).send()?;
        if !resp.status().is_success() {
            println!("Page {} nonexistent!", current);
            break;
        }
        let resp_html = delete_parentheses(resp.text()?);

        //et mut resp_html = r"<html><body>nihaoma?</body></html>";
        let doc = kuchiki::parse_html().one(resp_html);

        let mut link_found = false;

        for node_ref in doc.select("section > p > a[rel='mw:WikiLink']").unwrap() {
            let node = node_ref.as_node();
            // millions of unwrap() and borrow()
            let node_el = node.as_element().unwrap().borrow();
            let attrs = node_el.attributes.borrow();

            current = attrs.get("href").borrow().unwrap().to_string()[2..].to_string();
            //println!("{}", current);

            if !current.contains("Help:") && !current.contains("Template:") {
                link_found = true;
                break;
            }
        }
        if !link_found {
            println!("Deadend: No wikilink found on {}", current);
            break;
        }

        count += 1;
        if count >= 500 {
            break;
        }
    }
    Ok(())
}
fn main() {
    println!("--- THE WIKIPEDIA SCRAPER ---");
    match follow_first_links("Science", "Philosophy") {
        Ok(_) => (),
        Err(e) => println!("AIYAA! an error:\n{}", e)
    };
}