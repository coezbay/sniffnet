use std::sync::{Arc, Mutex};

use chrono::Local;
use etherparse::{IpHeader, TransportHeader};

use crate::{AppProtocol, InfoTraffic, IpVersion, TransProtocol};
use crate::enums::app_protocol::from_port_to_application_protocol;
use crate::enums::traffic_type::TrafficType;
use crate::structs::address_port_pair::AddressPortPair;
use crate::structs::info_address_port_pair::InfoAddressPortPair;

/// This function analyzes the network layer header passed as parameter and updates variables
/// passed by reference on the basis of the packet header content.
pub fn analyze_network_header(network_header: Option<IpHeader>, exchanged_bytes: &mut u128,
                              network_protocol: &mut IpVersion, address1: &mut String,
                              address2: &mut String, skip_packet: &mut bool) {
    match network_header {
        Some(IpHeader::Version4(ipv4header, _)) => {
            *network_protocol = IpVersion::IPv4;
            *address1 = format!("{:?}", ipv4header.source)
                .replace('[', "")
                .replace(']', "")
                .replace(',', ".")
                .replace(' ', "");
            *address2 = format!("{:?}", ipv4header.destination)
                .replace('[', "")
                .replace(']', "")
                .replace(',', ".")
                .replace(' ', "");
            *exchanged_bytes = ipv4header.payload_len as u128;
        }
        Some(IpHeader::Version6(ipv6header, _)) => {
            *network_protocol = IpVersion::IPv6;
            *address1 = ipv6_from_long_dec_to_short_hex(ipv6header.source);
            *address2 = ipv6_from_long_dec_to_short_hex(ipv6header.destination);
            *exchanged_bytes = ipv6header.payload_length as u128;
        }
        _ => {
            *skip_packet = true;
        }
    }
}


/// This function analyzes the transport layer header passed as parameter and updates variables
/// passed by reference on the basis of the packet header content.
pub fn analyze_transport_header(transport_header: Option<TransportHeader>,
                                port1: &mut u16, port2: &mut u16,
                                application_protocol: &mut AppProtocol,
                                transport_protocol: &mut TransProtocol, skip_packet: &mut bool) {
    match transport_header {
        Some(TransportHeader::Udp(udp_header)) => {
            *port1 = udp_header.source_port;
            *port2 = udp_header.destination_port;
            *transport_protocol = TransProtocol::UDP;
            *application_protocol = from_port_to_application_protocol(*port1);
            if (*application_protocol).eq(&AppProtocol::Other) {
                *application_protocol = from_port_to_application_protocol(*port2);
            }
        }
        Some(TransportHeader::Tcp(tcp_header)) => {
            *port1 = tcp_header.source_port;
            *port2 = tcp_header.destination_port;
            *transport_protocol = TransProtocol::TCP;
            *application_protocol = from_port_to_application_protocol(*port1);
            if (*application_protocol).eq(&AppProtocol::Other) {
                *application_protocol = from_port_to_application_protocol(*port2);
            }
        }
        _ => {
            *skip_packet = true;
        }
    }
}


/// Function to insert the source and destination of a packet into the shared map containing the analyzed traffic.
pub fn modify_or_insert_in_map(info_traffic_mutex: Arc<Mutex<InfoTraffic>>,
                               key: AddressPortPair, exchanged_bytes: u128,
                               traffic_type: TrafficType, application_protocol: AppProtocol) {
    let now = Local::now().to_string().get(0..19).unwrap().to_string();
    let trans_protocol = key.trans_protocol;
    let very_long_address = key.address1.len() > 25 || key.address2.len() > 25;
    let mut info_traffic = info_traffic_mutex.lock().expect("Error acquiring mutex\n\r");
    let len = info_traffic.map.len();
    let index = info_traffic.map.get_index_of(&key).unwrap_or(len);
    info_traffic.map.entry(key).and_modify(|info| {
        info.transmitted_bytes += exchanged_bytes;
        info.transmitted_packets += 1;
        info.final_timestamp = now.clone();
    })
        .or_insert(InfoAddressPortPair {
            transmitted_bytes: exchanged_bytes,
            transmitted_packets: 1,
            initial_timestamp: now.clone(),
            final_timestamp: now,
            trans_protocol,
            app_protocol: application_protocol,
            very_long_address,
            traffic_type,
        });
    info_traffic.addresses_last_interval.insert(index);
}


/// Determines if the input address is a multicast address or not.
///
/// # Arguments
///
/// * `address` - string representing an IPv4 or IPv6 network address.
pub fn is_multicast_address(address: &str) -> bool {
    let mut ret_val = false;
    if address.contains(':') { //IPv6 address
        if address.starts_with("ff") {
            ret_val = true;
        }
    } else { //IPv4 address
        let first_group = address.split('.').next().unwrap().to_string().parse::<u8>().unwrap();
        if (224..=239).contains(&first_group) {
            ret_val = true;
        }
    }
    ret_val
}


// Test for this function at the end of this file (run with cargo test)
/// Function to convert a long decimal ipv6 address to a
/// shorter compressed ipv6 address
///
/// # Arguments
///
/// * `ipv6_long` - Contains the 16 integer composing the not compressed decimal ipv6 address
///
/// # Example
///
/// ```
/// let result = ipv6_from_long_dec_to_short_hex([255,10,10,255,0,0,0,0,28,4,4,28,255,1,0,0]);
/// assert_eq!(result, "ff0a:aff::1c04:41c:ff01:0".to_string());
/// ```
pub fn ipv6_from_long_dec_to_short_hex(ipv6_long: [u8; 16]) -> String {

    //from hex to dec, paying attention to the correct number of digits
    let mut ipv6_hex = "".to_string();
    for i in 0..=15 {
        //pari: primo byte del gruppo
        if i % 2 == 0 {
            if *ipv6_long.get(i).unwrap() == 0 {
                continue;
            }
            ipv6_hex.push_str(&format!("{:x}", ipv6_long.get(i).unwrap()));
        }

        //dispari: secondo byte del gruppo
        else if *ipv6_long.get(i - 1).unwrap() == 0 {
            ipv6_hex.push_str(&format!("{:x}:", ipv6_long.get(i).unwrap()));
        } else {
            ipv6_hex.push_str(&format!("{:02x}:", ipv6_long.get(i).unwrap()));
        }
    }
    ipv6_hex.pop();

    // search for the longest zero sequence in the ipv6 address
    let mut to_compress: Vec<&str> = ipv6_hex.split(':').collect();
    let mut longest_zero_sequence = 0; // max number of consecutive zeros
    let mut longest_zero_sequence_start = 0; // first index of the longest sequence of zeros
    let mut current_zero_sequence = 0;
    let mut current_zero_sequence_start = 0;
    let mut i = 0;
    for s in to_compress.clone() {
        if s.eq("0") {
            if current_zero_sequence == 0 {
                current_zero_sequence_start = i;
            }
            current_zero_sequence += 1;
        } else if current_zero_sequence != 0 {
            if current_zero_sequence > longest_zero_sequence {
                longest_zero_sequence = current_zero_sequence;
                longest_zero_sequence_start = current_zero_sequence_start;
            }
            current_zero_sequence = 0;
        }
        i += 1;
    }
    if current_zero_sequence != 0 { // to catch consecutive zeros at the end
        if current_zero_sequence > longest_zero_sequence {
            longest_zero_sequence = current_zero_sequence;
            longest_zero_sequence_start = current_zero_sequence_start;
        }
    }
    if longest_zero_sequence < 2 { // no compression needed
        return ipv6_hex;
    }

    //from longest sequence of consecutive zeros to '::'
    let mut ipv6_hex_compressed = "".to_string();
    for _ in 0..longest_zero_sequence {
        to_compress.remove(longest_zero_sequence_start);
    }
    i = 0;
    if longest_zero_sequence_start == 0 {
        ipv6_hex_compressed.push_str("::");
    }
    for s in to_compress {
        ipv6_hex_compressed.push_str(s);
        ipv6_hex_compressed.push(':');
        i += 1;
        if i == longest_zero_sequence_start {
            ipv6_hex_compressed.push(':');
        }
    }
    if ipv6_hex_compressed.ends_with("::") {
        return ipv6_hex_compressed;
    }
    ipv6_hex_compressed.pop();

    ipv6_hex_compressed
}


#[cfg(test)]
mod ipv6_format_tests {
    use crate::utility::manage_packets::ipv6_from_long_dec_to_short_hex;

    #[test]
    fn simple_test() {
        let result = ipv6_from_long_dec_to_short_hex([255, 10, 10, 255, 255, 10, 10, 255, 255, 10, 10, 255, 255, 10, 10, 255, ]);
        assert_eq!(result, "ff0a:aff:ff0a:aff:ff0a:aff:ff0a:aff".to_string());
    }

    #[test]
    fn zeros_in_the_middle() {
        let result = ipv6_from_long_dec_to_short_hex([255, 10, 10, 255, 0, 0, 0, 0, 28, 4, 4, 28, 255, 1, 0, 0]);
        assert_eq!(result, "ff0a:aff::1c04:41c:ff01:0".to_string());
    }

    #[test]
    fn leading_zeros() {
        let result = ipv6_from_long_dec_to_short_hex([0, 0, 0, 0, 0, 0, 0, 0, 28, 4, 4, 28, 255, 1, 0, 10]);
        assert_eq!(result, "::1c04:41c:ff01:a".to_string());
    }

    #[test]
    fn tail_one_after_zeros() {
        let result = ipv6_from_long_dec_to_short_hex([28, 4, 4, 28, 255, 1, 0, 10, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(result, "1c04:41c:ff01:a::1".to_string());
    }

    #[test]
    fn tail_zeros() {
        let result = ipv6_from_long_dec_to_short_hex([28, 4, 4, 28, 255, 1, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(result, "1c04:41c:ff01:a::".to_string());
    }

    #[test]
    fn multiple_zero_sequences_first_longer() {
        let result = ipv6_from_long_dec_to_short_hex([32, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1]);
        assert_eq!(result, "2000::101:0:0:1".to_string());
    }

    #[test]
    fn multiple_zero_sequences_first_longer_head() {
        let result = ipv6_from_long_dec_to_short_hex([0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1]);
        assert_eq!(result, "::101:0:0:1".to_string());
    }

    #[test]
    fn multiple_zero_sequences_second_longer() {
        let result = ipv6_from_long_dec_to_short_hex([1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 3, 118]);
        assert_eq!(result, "100:0:0:1::376".to_string());
    }

    #[test]
    fn multiple_zero_sequences_second_longer_tail() {
        let result = ipv6_from_long_dec_to_short_hex([32, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(result, "2000:0:0:1:101::".to_string());
    }

    #[test]
    fn multiple_zero_sequences_equal_length() {
        let result = ipv6_from_long_dec_to_short_hex([118, 3, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 1]);
        assert_eq!(result, "7603::1:101:0:0:1".to_string());
    }

    #[test]
    fn all_zeros() {
        let result = ipv6_from_long_dec_to_short_hex([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(result, "::".to_string());
    }

    #[test]
    fn x_all_zeros() {
        let result = ipv6_from_long_dec_to_short_hex([161, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(result, "a100::".to_string());
    }

    #[test]
    fn all_zeros_x() {
        let result = ipv6_from_long_dec_to_short_hex([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 176]);
        assert_eq!(result, "::b0".to_string());
    }

    #[test]
    fn many_zeros_but_no_compression() {
        let result = ipv6_from_long_dec_to_short_hex([0, 16, 16, 0, 0, 1, 7, 0, 0, 2, 216, 0, 1, 0, 0, 1]);
        assert_eq!(result, "10:1000:1:700:2:d800:100:1".to_string());
    }
}