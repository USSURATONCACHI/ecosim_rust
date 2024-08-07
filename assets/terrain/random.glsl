// By Veniamin Kamnev (attiny13a_pu)

uint uhash2(uvec2 s) {
    uvec4 s1;
    s1 = (uvec4(s >> 16u, s & 0xFFFFu) * uvec4(0x7D202CFBu, 0xEDA6A77Du, 0x43EF69ABu, 0xE5C5A9ADu)) +
    uvec4(0x61C65DE7u, 0x7A0F89EFu, 0x8AF12C51u, 0x927E0E2Bu);

    s1 = (uvec4((s1.xz ^ s1.yw) >> 16u, (s1.xz ^ s1.yw) & 0xFFFFu) * uvec4(0x11028C59u, 0xFDA77C39u, 0x26783951u, 0x15A4DBB7u)) +
    uvec4(0xA5041D0Du, 0x27AE1933u, 0xDC1CA48Du, 0x577AE491u);

    s1 = (uvec4((s1.xy ^ s1.zw) >> 16u, (s1.xy ^ s1.zw) & 0xFFFFu) * uvec4(0x0FF1738Du, 0x6A5A87E1u, 0xED8C6B77u, 0xE97B7CC1u)) +
    uvec4(0xFFABDEAFu, 0xCFA02E1Fu, 0x401BE42Fu, 0x8E7195F1u);

    s1 = (uvec4((s1.xz ^ s1.yw) >> 16u, (s1.xz ^ s1.yw) & 0xFFFFu) * uvec4(0x486E046Du, 0xAA219B31u, 0x645CF729u, 0x384865D9u))
    + uvec4(0xA56EC0FBu, 0xBA8225C3u, 0xAC8003F3u, 0xCC7C86F7u);

    return ((((s1.x * 0xE7A2CA7Bu) ^ (s1.y * 0xB294EB91u)) * 0xEA1C1AF9u) ^
    (((s1.z * 0x6D95A9B9u) ^ (s1.w * 0x227A3011u)) * 0x9EE8315Bu)) * 0xC4830579u;
}

uvec2 u2hash2(uvec2 s) {
    uvec4 s1;
    s1 = (uvec4(s >> 16u, s & 0xFFFFu) * uvec4(0x404B6841u, 0xE48E763Du, 0xABDDB121u, 0x572F50FBu)) +
    uvec4(0x8C10CAE9u, 0x5C08C39Fu, 0xF30C9AE7u, 0xD1CC61D7u);

    s1 = (uvec4((s1.xz ^ s1.yw) >> 16u, (s1.xz ^ s1.yw) & 0xFFFFu) * uvec4(0x4E83008Fu, 0x8E4018D9u, 0x0DF8B3A3u, 0x3943F6B5u)) +
    uvec4(0x9EBAE1ADu, 0x8C58F83Bu, 0x2DC1DB45u, 0x785F6D2Bu);

    s1 = (uvec4((s1.xy ^ s1.zw) >> 16u, (s1.xy ^ s1.zw) & 0xFFFFu) * uvec4(0xF55C3365u, 0x905273D3u, 0x08CD92B3u, 0x887CFDC5u)) +
    uvec4(0xB57C6885u, 0xA619CD09u, 0x3C1DB35Du, 0x79EB6549u);

    s1 = (uvec4((s1.xz ^ s1.yw) >> 16u, (s1.xz ^ s1.yw) & 0xFFFFu) * uvec4(0x78F38F33u, 0xC9B48A87u, 0xD2854EE5u, 0xCE985B49u)) +
    uvec4(0x6D95A9B9u, 0xD7B87323u, 0x61BF7D4Du, 0xE4857E25u);

    return uvec2(s1.x ^ s1.z, s1.y ^ s1. w) * uvec2(0xCA5333C9u, 0x02BDCF69u);
}

uint uhash4(uvec4 s) {
    uvec4 s1, s2;

    s1 = (uvec4(s.zw >> 16u, s.zw & 0xFFFFu) * uvec4(0x71217B47u, 0x87E9615Fu, 0xBA96E469u, 0x9F7AFBB5u)) +
    uvec4(0x41526BCDu, 0x8D1C8F5Du, 0x340B0C59u, 0x51AB5713u);

    s1 = (uvec4((s1.xz ^ s1.yw) >> 16u, (s1.xz ^ s1.yw) & 0xFFFFu) * uvec4(0xB7B5D5DBu, 0x35C75427u, 0x982CC8CFu, 0x51824591u)) +
    uvec4(0x6B2FDB87u, 0x28232A37u, 0x1EC27BDFu, 0x8DC8079Fu) ^ (s);

    s1 = (uvec4((s1.xy ^ s1.zw) >> 16u, (s1.xy ^ s1.zw) & 0xFFFFu) * uvec4(0x3795BCB5u, 0xC2BFF81Bu, 0xA05194E9u, 0xAA48F4E5u)) +
    uvec4(0x6084455Fu, 0xAFB852D5u, 0x84973225u, 0x4D17B761u);

    s1 = (uvec4((s1.xz ^ s1.yw) >> 16u, (s1.xz ^ s1.yw) & 0xFFFFu) * uvec4(0xD2D96441u, 0x3B76C561u, 0x5C597335u, 0xCC82F905u)) +
    uvec4(0xAFF8F95Du, 0x05DDA739u, 0x8D2AF67Fu, 0xF6E649B3u);


    s2 = (uvec4(s.xy >> 16u, s.xy & 0xFFFFu) * uvec4(0xD1358937u, 0x3FA29D3Du, 0xE668FCF9u, 0x9F9D257Fu)) +
    uvec4(0xC2F41E4Du, 0x067B1B8Du, 0x2AB52157u, 0x78321A05u);

    s2 = (uvec4((s2.xz ^ s2.yw) >> 16u, (s2.xz ^ s2.yw) & 0xFFFFu) * uvec4(0x1760A9B1u, 0xD53DF509u, 0xC7DDC9D9u, 0xD08AF30Bu)) +
    uvec4(0xC699F1A1u, 0x9C9885C7u, 0x2F71BC5Fu, 0x98B1D685u) ^ (s);

    s2 = (uvec4((s2.xy ^ s2.zw) >> 16u, (s2.xy ^ s2.zw) & 0xFFFFu) * uvec4(0x821DA417u, 0x15D81063u, 0x15FADA2Fu, 0xC0F5B591u)) +
    uvec4(0x3F8FA15Fu, 0x0A2818FDu, 0x104EEA71u, 0x2F060EE1u);

    s2 = (uvec4((s2.xz ^ s2.yw) >> 16u, (s2.xz ^ s2.yw) & 0xFFFFu) * uvec4(0x51BFC0FBu, 0x369D933Fu, 0x0EFDF55Fu, 0xCD5BA4D1u)) +
    uvec4(0x3C320C3Fu, 0xF4D45287u, 0x87F4294Du, 0x2738D7C9u);

    return ((((s1.x ^ s2.x) * 0xB462A63Du) ^ ((s1.y ^ s2.y) * 0xAA13ED13u) * 0x2F1FEEBBu) ^
    (((s1.z ^ s2.z) * 0xE89238CBu) ^ ((s1.w ^ s2.w) * 0xA0E55C97u) * 0x143FB695u)) * 0xFB8EEDC7u;
}
