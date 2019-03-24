use kuchiki::traits::*;
use std::borrow::Borrow;
use std::collections::{HashMap};
use regex::{Regex};
use unicode_segmentation::UnicodeSegmentation;

fn word_analysis(title: &str) -> Result<(), Box<std::error::Error>> {
    println!("Fetching: {}...", title);
    let page_url = title;
    // TODO replace spaces with underscores

    let base_url = url::Url::parse("https://en.wikipedia.org/api/rest_v1/page/html/").unwrap();
    let mut resp = reqwest::get(base_url.join(page_url).unwrap())?;
    let resp_html = resp.text()?;
    let doc = kuchiki::parse_html().one(resp_html);
    
    let mut nodes_to_delete = vec![];
    // NOTE: these must be placed in a vec because if we detach it in the loop
    // the loop only runs once, due to unintended behaviour!
    for node_ref in doc.select("section > p > sup").unwrap() {
        nodes_to_delete.push(node_ref);
    }
    for n in nodes_to_delete {
        n.as_node().detach();
    }
    let mut text: String = "".to_string();
    
    for node_ref in doc.select("section > p").unwrap() {
        //print
        let p = node_ref.text_contents();
        text += &p;
        // let re = regex::Regex::new(r" [\[\]()]").unwrap();
        // let words = re.replace_all(s, re);
    }
    println!("{}", text);
    let stats = word_count(text);
    for (k, count) in stats.word_counts.iter() {
        let word_variants = stats.word_variants.get(k).unwrap();
        let word_entry = word_variants.join("/");
        println!("{}: {}", word_entry, count);
    }
    Ok(())
}

struct TextStatistics {
    word_counts: HashMap<String, u32>,
    word_variants: HashMap<String, Vec<String>>,
}
fn word_count(text: String) -> TextStatistics {
    let mut stats = TextStatistics {
        word_counts: HashMap::new(),
        word_variants: HashMap::new(),
    };
    // Latin
    let latin = r"[A-Za-zÀ-ÖØ-öø-ÿ]";
    // match latin/dash/apostrophe
    let re = Regex::new(&format!(r"{0}[{0}'\-]+", latin)).unwrap();
    for cap in re.captures_iter(&text) {
        let word = cap[0].to_string();
        print!("{} ", word);
        let lower = word.to_lowercase(); // unicode supported
        
        let word_count = stats.word_counts.entry(lower.clone()).or_insert(0);
        *word_count += 1;

        let variants = stats.word_variants.entry(lower).or_insert(Vec::new());
        if !variants.contains(&word) {
            variants.push(word);
        }
    }

    stats
}

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
    // the rest_v1 html has lots of additional metadata
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
            // millions of unwrap()s and borrow()s
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
    // match follow_first_links("Science", "Philosophy") {
    //     Ok(_) => (),
    //     Err(e) => println!("AIYAA! an error:\n{}", e)
    // };
    word_analysis("Jens Stub");
}