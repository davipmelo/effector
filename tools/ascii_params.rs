    .summary("Rebuilds the image from characters.")
    .params(vec![
        ParamDef::choice_opts("charset", "Characters", CHARSETS.iter().map(|c| opt(c.0, c.1)).collect(), "standard").sec("Characters"),
        ParamDef::text("customChars", "Your Characters (dark → light order is automatic)", " .oO@")
            .sec("Characters")
            .vis(when("charset", "custom")),
        s("cell", "Cell Size", 2.0, 40.0, 8.0).step(0.5).curve(1.6).sec("Characters"),
        s("aspect", "Cell Height", 100.0, 250.0, 180.0).step(1.0).unit("%").sec("Characters"),
        ParamDef::toggle("bold", "Bold", false).sec("Characters"),
        ParamDef::segmented("mode", "Color", &[("mono", "Mono"), ("palette", "Palette"), ("original", "Original")], "mono").sec("Color"),
        ParamDef::color("fg", "Characters", hex("#F2F2F2")).sec("Color").vis(when("mode", "mono")),
        ParamDef::color("bg", "Background", hex("#0A0A0A")).sec("Color"),
        ParamDef::palette("palette", "Palette", thermal_palette()).sec("Color").vis(when("mode", "palette")),
        ParamDef::toggle("invert", "Invert", false).sec("Tone"),
        s("contrast", "Contrast", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
        s("brightness", "Brightness", -100.0, 100.0, 0.0).step(1.0).sec("Tone"),
    ])
    .looks(vec![
        look("terminal", "Green Terminal", vals!["charset" => "standard", "fg" => hex("#5CFF7A"), "bg" => hex("#04110A")]),
        look("amber", "Amber Terminal", vals!["charset" => "detailed", "cell" => 6.0, "fg" => hex("#FFB000"), "bg" => hex("#140A00")]),
        look("typewriter", "Typewriter", vals!["charset" => "detailed", "cell" => 6.0, "invert" => true, "fg" => hex("#1A1A1A"), "bg" => hex("#F2F2F2")]),
        look("matrix", "Matrix", vals!["charset" => "matrix", "cell" => 9.0, "fg" => hex("#3BFF6E"), "bg" => hex("#000000"), "contrast" => 40.0]),
        look("color", "Color Text", vals!["charset" => "detailed", "cell" => 5.0, "mode" => "original", "bg" => hex("#000000")]),
        look("blocks", "Block Art", vals!["charset" => "blocks", "cell" => 7.0, "aspect" => 100.0, "mode" => "original"]),
        look("braille", "Braille Fine", vals!["charset" => "braille", "cell" => 4.0, "aspect" => 150.0]),
        look("binary", "Binary Code", vals!["charset" => "binary", "cell" => 8.0, "fg" => hex("#29ADFF"), "bg" => hex("#020812")]),
        look("thermal", "Thermal Text", vals!["charset" => "standard", "mode" => "palette", "cell" => 7.0, "palette" => thermal_palette()]),
        look("yellow", "EFFECTOR", vals!["charset" => "letters", "cell" => 8.0, "fg" => hex("#FFBB00"), "bg" => hex("#0A0A0A")]),
    ])
