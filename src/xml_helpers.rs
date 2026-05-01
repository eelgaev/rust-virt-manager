use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Debug, Clone)]
pub struct XmlSpan {
    pub start: usize,
    pub end: usize,
    pub content: String,
}

fn pos(reader: &Reader<&[u8]>) -> usize {
    reader.buffer_position() as usize
}

pub fn find_nth_device_element(domain_xml: &str, tag_name: &str, n: usize) -> Option<XmlSpan> {
    let devices_start = domain_xml.find("<devices")?;
    let devices_close = domain_xml.find("</devices>")?;
    let section = &domain_xml[devices_start..devices_close + 10];

    let mut reader = Reader::from_str(section);
    let mut depth: i32 = 0;
    let mut count: usize = 0;
    let mut capture_start: Option<usize> = None;
    let mut capture_depth: i32 = 0;
    let tag = tag_name.as_bytes();

    loop {
        let event_offset = pos(&reader);
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                depth += 1;
                if depth == 2 && e.name().as_ref() == tag && capture_start.is_none() {
                    if count == n {
                        capture_start = Some(event_offset);
                        capture_depth = 1;
                    } else {
                        count += 1;
                    }
                } else if capture_start.is_some() {
                    capture_depth += 1;
                }
            }
            Ok(Event::End(e)) => {
                if capture_start.is_some() && e.name().as_ref() == tag {
                    capture_depth -= 1;
                    if capture_depth == 0 {
                        let s = capture_start.unwrap();
                        let end = pos(&reader);
                        let abs_s = devices_start + s;
                        let abs_e = devices_start + end;
                        return Some(XmlSpan {
                            start: abs_s,
                            end: abs_e,
                            content: domain_xml[abs_s..abs_e].to_string(),
                        });
                    }
                }
                depth -= 1;
            }
            Ok(Event::Empty(e)) => {
                if depth == 1 && e.name().as_ref() == tag {
                    if count == n {
                        let abs_s = devices_start + event_offset;
                        let abs_e = devices_start + pos(&reader);
                        return Some(XmlSpan {
                            start: abs_s,
                            end: abs_e,
                            content: domain_xml[abs_s..abs_e].to_string(),
                        });
                    }
                    count += 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    None
}

pub fn replace_nth_device_element(
    domain_xml: &str,
    tag_name: &str,
    n: usize,
    new_xml: &str,
) -> Option<String> {
    let span = find_nth_device_element(domain_xml, tag_name, n)?;
    let mut result = String::with_capacity(domain_xml.len());
    result.push_str(&domain_xml[..span.start]);
    result.push_str(new_xml);
    result.push_str(&domain_xml[span.end..]);
    Some(result)
}

pub fn remove_nth_device_element(domain_xml: &str, tag_name: &str, n: usize) -> Option<String> {
    let span = find_nth_device_element(domain_xml, tag_name, n)?;
    let mut result = String::with_capacity(domain_xml.len());
    let before = &domain_xml[..span.start];
    let after = &domain_xml[span.end..];
    result.push_str(before.trim_end_matches(|c: char| c == ' ' || c == '\t'));
    let trimmed = after.trim_start_matches(|c: char| c == ' ' || c == '\t');
    if trimmed.starts_with('\n') {
        result.push_str(&trimmed[1..]);
    } else {
        result.push_str(trimmed);
    }
    Some(result)
}

pub fn find_toplevel_element(domain_xml: &str, tag_name: &str) -> Option<XmlSpan> {
    let mut reader = Reader::from_str(domain_xml);
    let mut depth: i32 = 0;
    let tag = tag_name.as_bytes();
    let mut capture_start: Option<usize> = None;
    let mut capture_depth: i32 = 0;

    loop {
        let event_offset = pos(&reader);
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                depth += 1;
                if depth == 2 && e.name().as_ref() == tag && capture_start.is_none() {
                    capture_start = Some(event_offset);
                    capture_depth = 1;
                } else if capture_start.is_some() {
                    capture_depth += 1;
                }
            }
            Ok(Event::End(e)) => {
                if capture_start.is_some() && e.name().as_ref() == tag {
                    capture_depth -= 1;
                    if capture_depth == 0 {
                        let s = capture_start.unwrap();
                        let end = pos(&reader);
                        return Some(XmlSpan {
                            start: s,
                            end,
                            content: domain_xml[s..end].to_string(),
                        });
                    }
                }
                depth -= 1;
            }
            Ok(Event::Empty(e)) => {
                if depth == 1 && e.name().as_ref() == tag {
                    let abs_s = event_offset;
                    let abs_e = pos(&reader);
                    return Some(XmlSpan {
                        start: abs_s,
                        end: abs_e,
                        content: domain_xml[abs_s..abs_e].to_string(),
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    None
}

pub fn replace_toplevel_element(
    domain_xml: &str,
    tag_name: &str,
    new_xml: &str,
) -> Option<String> {
    let span = find_toplevel_element(domain_xml, tag_name)?;
    let mut result = String::with_capacity(domain_xml.len());
    result.push_str(&domain_xml[..span.start]);
    result.push_str(new_xml);
    result.push_str(&domain_xml[span.end..]);
    Some(result)
}
