use std::io::Read;
use std::net::{IpAddr, SocketAddr};

use bytes::{Buf, BufMut, BytesMut};

use crate::error::VexResult;
use crate::instance::{IPV4_LOCAL_ADDR, IPV6_LOCAL_ADDR};
use crate::network::raknet::{Frame, FrameBatch, Header, OrderChannel, Reliability};
use crate::network::raknet::packets::{Decodable, NewIncomingConnection, OnlinePing};
use crate::util::{ReadExtensions, WriteExtensions};

#[test]
fn read_write_var_u32() {
    let mut buffer = BytesMut::new();
    buffer.put_var_u32(45);
    buffer.put_var_u32(2769);
    buffer.put_var_u32(105356);
    buffer.put_var_u32(359745976);

    let mut buffer = buffer.freeze();
    assert_eq!(buffer.get_var_u32().unwrap(), 45);
    assert_eq!(buffer.get_var_u32().unwrap(), 2769);
    assert_eq!(buffer.get_var_u32().unwrap(), 105356);
    assert_eq!(buffer.get_var_u32().unwrap(), 359745976);

    let mut buffer = BytesMut::from([0xc1, 0xe9, 0x33].as_ref());
    let a = buffer.get_var_u32().unwrap();

    let mut buffer2 = BytesMut::new();
    buffer2.put_var_u32(a);

    assert_eq!(&[0xc1, 0xe9, 0x33], buffer2.as_ref());
}

#[test]
fn read_write_u24_le() {
    let mut buffer = BytesMut::new();
    buffer.put_u24_le(125); // Test first byte only
    buffer.put_u24_le(50250); // Test first two bytes
    buffer.put_u24_le(1097359); // Test all bytes

    let mut buffer = buffer.freeze();
    assert_eq!(buffer.get_u24_le(), 125);
    assert_eq!(buffer.get_u24_le(), 50250);
    assert_eq!(buffer.get_u24_le(), 1097359);
}

#[test]
fn read_write_addr() -> VexResult<()> {
    let ipv4_test = SocketAddr::new(IpAddr::V4(IPV4_LOCAL_ADDR), 19132);
    let ipv6_test = SocketAddr::new(IpAddr::V6(IPV6_LOCAL_ADDR), 19133);

    let mut buffer = BytesMut::new();
    buffer.put_addr(ipv4_test); // Test IPv4
    buffer.put_addr(ipv6_test); // Test IPv6

    let mut buffer = buffer.freeze();
    assert_eq!(buffer.get_addr()?, ipv4_test);
    assert_eq!(buffer.get_addr()?, ipv6_test);
    Ok(())
}

#[test]
fn order_channel() {
    let mut test_frame = Frame::default();
    let mut channel = OrderChannel::new();

    test_frame.order_index = 0;
    assert!(channel.insert(test_frame.clone()).is_some());

    test_frame.order_index = 2;
    assert!(channel.insert(test_frame.clone()).is_none());

    test_frame.order_index = 1;
    let output = channel.insert(test_frame).unwrap();

    assert_eq!(output.len(), 2);
    assert_eq!(output[0].order_index, 1);
    assert_eq!(output[1].order_index, 2);
}

#[test]
fn test() {
    /// Length: 0x0c (12)
    /// ID: 0x8f (NetworkSettings)
    /// Compression threshold: 1 (always compress),
    /// Compression algorithm: 1 (Snappy),
    /// Client throttling disabled
    /// Client throttle threshold: 0
    /// Client throttle scalar: 0
    const NETWORK_SETTINGS_DUMP: &[u8] = &[
        /*0x02, 0x00, 0x00, 0x00, 0x45, 0x00, 0x00, 0x38, 0x93, 0x77, 0x00, 0x00, 0x80, 0x11, 0x00, 0x00,
        0x7f, 0x00, 0x00, 0x01, 0x7f, 0x00, 0x00, 0x01, 0x4a, 0xbc, 0xda, 0x11, 0x00, 0x24, 0xf9, 0xc1,
        0x84, 0x03, 0x00, 0x00, 0x60, 0x00, 0x70, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, */
        0xfe, 0x0c, 0x8f, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    /// Compressed using Snappy, sent immediately after Login
    /// Not fragmented
    const DUMP: &[u8] = &[
        /*0x02, 0x00, 0x00, 0x00, 0x45, 0x00, 0x01, 0x92, 0x93, 0x9a, 0x00, 0x00, 0x80, 0x11, 0x00, 0x00,
        0x7f, 0x00, 0x00, 0x01, 0x7f, 0x00, 0x00, 0x01, 0x4a, 0xbc, 0xda, 0x11, 0x01, 0x7e, 0xbc, 0x49,
        0x84, 0x05, 0x00, 0x00, 0x60, 0x0b, 0x40, 0x02, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0xfe, */
        0x0d, 0xcf, 0xdb, 0x92, 0x9a, 0x30, 0x00, 0x00, 0xd0, 0x07, 0xbf, 0xa7, 0x3b, 0x09, 0x85,
        0x5d, 0x79, 0x14, 0x25, 0x98, 0xd0, 0xc4, 0x72, 0x09, 0x90, 0xbc, 0xec, 0x40, 0xc0, 0x82,
        0x84, 0x8b, 0x05, 0x57, 0xcc, 0x1f, 0x75, 0xa6, 0x1f, 0xd9, 0x9e, 0x3f, 0x38, 0x7f, 0x77,
        0xbb, 0x3f, 0xbb, 0xe6, 0x45, 0xda, 0x2a, 0x50, 0xdd, 0xa5, 0x23, 0x88, 0x1b, 0x6a, 0xb3,
        0x23, 0x5e, 0xf0, 0xf8, 0x0b, 0xd6, 0x09, 0x7e, 0xc7, 0x3d, 0xc4, 0x79, 0x51, 0xa3, 0x28,
        0xd3, 0x38, 0xb1, 0xdc, 0x32, 0x29, 0xe6, 0x3b, 0xf5, 0x99, 0xc7, 0x33, 0x1d, 0x24, 0x26,
        0x46, 0x31, 0x47, 0xa7, 0x84, 0xc7, 0x52, 0x02, 0x84, 0x1a, 0xbe, 0x59, 0xf4, 0x44, 0x21,
        0x2d, 0x74, 0x56, 0x1e, 0xdd, 0x53, 0x12, 0x38, 0xa0, 0x82, 0xb3, 0x53, 0xa6, 0x2c, 0x52,
        0x83, 0xbc, 0xa5, 0xd6, 0x1a, 0x85, 0xc0, 0x81, 0x72, 0x24, 0x38, 0xb4, 0xd6, 0x47, 0xd4,
        0x4f, 0x40, 0x05, 0xeb, 0xcf, 0xe8, 0x26, 0x7f, 0xa8, 0x7e, 0xfe, 0x0a, 0x21, 0x1a, 0xa9,
        0xdf, 0xb6, 0x14, 0x6a, 0xa4, 0x34, 0xf1, 0xd2, 0x53, 0xad, 0xc5, 0x18, 0x17, 0xa5, 0x4f,
        0x3c, 0xc1, 0x09, 0xaa, 0x7a, 0x92, 0x95, 0x85, 0x7e, 0x08, 0x0e, 0x1f, 0x51, 0x36, 0x77,
        0x71, 0x21, 0x53, 0xe6, 0x93, 0x67, 0xea, 0xb3, 0x4b, 0xd2, 0x2f, 0x36, 0xcf, 0xed, 0x67,
        0x33, 0x30, 0x1e, 0xc2, 0x39, 0x8c, 0x0b, 0xb4, 0x5c, 0x7c, 0xbe, 0xd1, 0x1c, 0x9d, 0xb3,
        0x22, 0xb6, 0x9b, 0x9c, 0x79, 0x6a, 0xc8, 0x06, 0x6a, 0xc4, 0x77, 0x9e, 0xf9, 0xdd, 0x35,
        0x9a, 0xde, 0xfe, 0xbf, 0x8d, 0xc8, 0x37, 0x80, 0x6f, 0x53, 0x17, 0xe9, 0xde, 0x54, 0x26,
        0x7e, 0x29, 0xc3, 0x9f, 0x69, 0xaa, 0xfb, 0xc8, 0x42, 0x20, 0xb3, 0x90, 0xa3, 0x02, 0x49,
        0x6a, 0x03, 0x5c, 0x3c, 0x82, 0xf0, 0x2d, 0x7f, 0x00, 0xc3, 0xf7, 0x4e, 0x2a, 0x9b, 0xc9,
        0x99, 0xea, 0x7a, 0x2d, 0xd9, 0xd6, 0xa3, 0xfc, 0xbe, 0x0a, 0xe7, 0x6a, 0x89, 0xa9, 0x58,
        0xf6, 0xb5, 0x0b, 0xd9, 0xf3, 0x63, 0x91, 0x6a, 0x43, 0xbf, 0x03, 0x57, 0x5d, 0x21, 0xb6,
        0xc4, 0x1a, 0x19, 0x33, 0x85, 0xe2, 0xa3, 0xed, 0xc9, 0xda, 0x36, 0x8e, 0x94, 0x65, 0x73,
        0x66, 0xe1, 0xed, 0x20, 0xcc, 0x14, 0x8c, 0x7b, 0xdc, 0xbe, 0x0e, 0x07, 0xdc, 0x5a, 0xc7,
        0x4d, 0x4c, 0x62, 0xfc, 0xfc, 0x9a, 0x47, 0x7b, 0x5f, 0xa5, 0xd6, 0x36, 0x6c, 0x9e, 0x73,
        0x3f, 0xdb, 0x72, 0x3b, 0x7e, 0xd3, 0x66, 0x3b, 0x83, 0x7b, 0x07, 0x07, 0xfa, 0x0f,
    ];

    const DECOMPRESSED: &[u8] = &[
        0xac, 0x3, 0x3, 0xa9, 0x3, 0x65, 0x79, 0x4a, 0x68, 0x62, 0x47, 0x63, 0x69, 0x4f, 0x69,
        0x4a, 0x46, 0x55, 0x7a, 0x4d, 0x34, 0x4e, 0x43, 0x49, 0x73, 0x49, 0x6e, 0x67, 0x31, 0x64,
        0x53, 0x49, 0x36, 0x49, 0x6b, 0x31, 0x49, 0x57, 0x58, 0x64, 0x46, 0x51, 0x56, 0x6c, 0x49,
        0x53, 0x32, 0x39, 0x61, 0x53, 0x58, 0x70, 0x71, 0x4d, 0x45, 0x4e, 0x42, 0x55, 0x56, 0x6c,
        0x47, 0x53, 0x7a, 0x52, 0x46, 0x52, 0x55, 0x46, 0x44, 0x53, 0x55, 0x52, 0x5a, 0x5a, 0x30,
        0x46, 0x46, 0x65, 0x55, 0x78, 0x32, 0x4d, 0x44, 0x4d, 0x31, 0x4d, 0x58, 0x6c, 0x56, 0x61,
        0x43, 0x39, 0x44, 0x53, 0x47, 0x35, 0x30, 0x62, 0x31, 0x70, 0x35, 0x61, 0x54, 0x4e, 0x51,
        0x63, 0x6d, 0x5a, 0x6a, 0x54, 0x32, 0x74, 0x51, 0x4b, 0x30, 0x35, 0x31, 0x5a, 0x6e, 0x4a,
        0x49, 0x4b, 0x32, 0x74, 0x75, 0x51, 0x6b, 0x6f, 0x30, 0x63, 0x47, 0x74, 0x50, 0x51, 0x6a,
        0x5a, 0x4c, 0x63, 0x6b, 0x70, 0x76, 0x4b, 0x31, 0x46, 0x6e, 0x4d, 0x45, 0x68, 0x68, 0x4d,
        0x31, 0x6c, 0x46, 0x63, 0x6c, 0x4a, 0x42, 0x54, 0x44, 0x64, 0x6c, 0x59, 0x6e, 0x52, 0x58,
        0x61, 0x45, 0x4a, 0x42, 0x59, 0x55, 0x4a, 0x46, 0x62, 0x6b, 0x4a, 0x56, 0x61, 0x58, 0x6c,
        0x75, 0x59, 0x55, 0x31, 0x75, 0x51, 0x56, 0x70, 0x69, 0x52, 0x58, 0x5a, 0x54, 0x4e, 0x45,
        0x4a, 0x77, 0x54, 0x45, 0x4e, 0x4f, 0x53, 0x6b, 0x73, 0x34, 0x55, 0x57, 0x34, 0x77, 0x65,
        0x6d, 0x4e, 0x55, 0x4b, 0x31, 0x70, 0x4b, 0x52, 0x58, 0x46, 0x73, 0x4f, 0x45, 0x55, 0x78,
        0x4d, 0x57, 0x46, 0x48, 0x56, 0x58, 0x52, 0x34, 0x65, 0x57, 0x4e, 0x42, 0x63, 0x6d, 0x56,
        0x6d, 0x4d, 0x7a, 0x59, 0x33, 0x55, 0x56, 0x45, 0x69, 0x66, 0x51, 0x6f, 0x2e, 0x65, 0x79,
        0x4a, 0x7a, 0x59, 0x57, 0x78, 0x30, 0x49, 0x6a, 0x6f, 0x69, 0x51, 0x6c, 0x6b, 0x7a, 0x62,
        0x7a, 0x52, 0x79, 0x63, 0x7a, 0x55, 0x77, 0x54, 0x54, 0x6c, 0x6b, 0x51, 0x32, 0x46, 0x30,
        0x56, 0x32, 0x46, 0x35, 0x63, 0x47, 0x5a, 0x4a, 0x64, 0x7a, 0x30, 0x39, 0x49, 0x6e, 0x30,
        0x4b, 0x2e, 0x57, 0x75, 0x30, 0x7a, 0x55, 0x38, 0x35, 0x54, 0x5a, 0x65, 0x6f, 0x35, 0x6f,
        0x64, 0x64, 0x74, 0x61, 0x4e, 0x78, 0x6b, 0x46, 0x57, 0x71, 0x74, 0x59, 0x35, 0x66, 0x32,
        0x59, 0x6f, 0x58, 0x73, 0x38, 0x64, 0x39, 0x31, 0x4e, 0x77, 0x37, 0x73, 0x5a, 0x63, 0x78,
        0x46, 0x72, 0x47, 0x39, 0x63, 0x66, 0x31, 0x49, 0x32, 0x59, 0x74, 0x51, 0x7a, 0x7a, 0x6f,
        0x4b, 0x59, 0x37, 0x68, 0x6b, 0x4a, 0x74, 0x68, 0x65, 0x35, 0x5a, 0x5a, 0x61, 0x65, 0x48,
        0x4e, 0x4b, 0x6a, 0x41, 0x59, 0x7a, 0x6f, 0x47, 0x6e, 0x38, 0x49, 0x68, 0x79, 0x41, 0x41,
        0x49, 0x68, 0x32, 0x43, 0x78, 0x59, 0x6f, 0x59, 0x6e, 0x5f, 0x76, 0x70, 0x6e, 0x34, 0x38,
        0x62, 0x54, 0x32, 0x78, 0x6d, 0x78, 0x42, 0x35, 0x71, 0x48, 0x34, 0x5a, 0x78, 0x43, 0x2d,
        0x6c, 0x7a, 0x78, 0x48, 0x30, 0x71, 0x69, 0x31, 0x6d, 0x4d,
    ];

    let mut buffer = BytesMut::from(DECOMPRESSED);
    let length = buffer.get_var_u32().unwrap();
    let header = Header::decode(&mut buffer).unwrap();

    println!("{header:?}");
}
