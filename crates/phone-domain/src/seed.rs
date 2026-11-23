use crate::*;

pub fn demo() -> PhoneData {
    let mut data = PhoneData::default();
    data.tiles = [
        (AppId::Phone, TileSize::Medium),
        (AppId::Messaging, TileSize::Medium),
        (AppId::People, TileSize::Medium),
        (AppId::Outlook, TileSize::Medium),
        (AppId::Photos, TileSize::Wide),
        (AppId::Calendar, TileSize::Medium),
        (AppId::InternetExplorer, TileSize::Medium),
        (AppId::Store, TileSize::Small),
        (AppId::Music, TileSize::Small),
        (AppId::Camera, TileSize::Small),
        (AppId::Settings, TileSize::Small),
        (AppId::Weather, TileSize::Wide),
        (AppId::OneNote, TileSize::Medium),
        (AppId::Games, TileSize::Medium),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (app, size))| Tile::new(100 + index as u64, app, size))
    .collect();
    data.contacts = [
        (
            1,
            "Alex Morgan",
            "+1 202 555 0101",
            "alex@example.com",
            "#D80073",
            true,
        ),
        (
            2,
            "Jamie Chen",
            "+1 202 555 0102",
            "jamie@example.com",
            "#00ABA9",
            true,
        ),
        (
            3,
            "Sam Wilson",
            "+1 202 555 0103",
            "sam@example.com",
            "#FA6800",
            true,
        ),
        (
            4,
            "Taylor Reed",
            "+1 202 555 0104",
            "taylor@example.com",
            "#6A00FF",
            false,
        ),
        (
            5,
            "Morgan Lee",
            "+1 202 555 0105",
            "morgan@example.com",
            "#60A917",
            false,
        ),
    ]
    .into_iter()
    .map(|(id, name, phone, email, color, favorite)| Contact {
        id,
        name: name.into(),
        phone: phone.into(),
        email: email.into(),
        color: color.into(),
        favorite,
        company: String::new(),
    })
    .collect();
    const NOW: i64 = 1_795_363_200;
    data.conversations = vec![
        Conversation {
            id: 201,
            contact_id: Some(1),
            recipient: "+1 202 555 0101".into(),
            unread: true,
            messages: vec![
                Message {
                    id: 211,
                    text: "Hey! Are we still on for coffee?".into(),
                    incoming: true,
                    timestamp: NOW - 7200,
                    draft: false,
                },
                Message {
                    id: 212,
                    text: "Absolutely. See you at 3!".into(),
                    incoming: false,
                    timestamp: NOW - 7000,
                    draft: false,
                },
                Message {
                    id: 213,
                    text: "Perfect, see you there ☺".into(),
                    incoming: true,
                    timestamp: NOW - 2400,
                    draft: false,
                },
            ],
        },
        Conversation {
            id: 202,
            contact_id: Some(2),
            recipient: "+1 202 555 0102".into(),
            unread: true,
            messages: vec![Message {
                id: 214,
                text: "The photos from our trip look amazing!".into(),
                incoming: true,
                timestamp: NOW - 8000,
                draft: false,
            }],
        },
        Conversation {
            id: 203,
            contact_id: Some(3),
            recipient: "+1 202 555 0103".into(),
            unread: false,
            messages: vec![Message {
                id: 215,
                text: "Thanks for the recommendation.".into(),
                incoming: true,
                timestamp: NOW - 86400,
                draft: false,
            }],
        },
    ];
    data.mail=vec![
        Mail{id:301,from:"Alex Morgan".into(),address:"alex@example.com".into(),subject:"Weekend plans".into(),body:"Hi there,\n\nHow about a walk by the lake this weekend? The weather looks perfect. Bring your camera!\n\nSee you soon,\nAlex\n\nSample message — this inbox is stored only on this device.".into(),timestamp:NOW-3600,read:false,flagged:false,folder:"inbox".into()},
        Mail{id:302,from:"Lumia".into(),address:"welcome@example.com".into(),subject:"Welcome to your phone".into(),body:"Hello,\n\nMake this phone yours. Press and hold a tile to move it, resize it, or unpin it. Swipe left on Start to find all your apps.\n\nYour changes are saved on this device. You can export a backup from Settings.\n\nEnjoy exploring.".into(),timestamp:NOW-86400,read:true,flagged:true,folder:"inbox".into()},
        Mail{id:303,from:"Jamie Chen".into(),address:"jamie@example.com".into(),subject:"A little inspiration".into(),body:"Found a great spot for our next photo walk. Let's compare notes over coffee.\n\nJamie".into(),timestamp:NOW-172800,read:true,flagged:false,folder:"inbox".into()},
    ];
    data.events = vec![
        CalendarEvent {
            id: 401,
            title: "Coffee with Alex".into(),
            date: "2026-11-22".into(),
            time: "15:00".into(),
            duration_minutes: 60,
            location: "The corner café".into(),
            details: "Bring the photos from our trip.".into(),
            ..CalendarEvent::default()
        },
        CalendarEvent {
            id: 402,
            title: "A walk by the lake".into(),
            date: "2026-11-23".into(),
            time: "10:00".into(),
            duration_minutes: 90,
            location: "Lakeside park".into(),
            details: String::new(),
            ..CalendarEvent::default()
        },
    ];
    data.notes=vec![
        Note{id:501,title:"A little inspiration".into(),body:"Places to go\n• A quiet trail through the forest\n• The coast at sunrise\n• Somewhere new\n\nThings to bring\nCamera, notebook, and a good playlist.".into(),modified:NOW},
        Note{id:502,title:"Shopping list".into(),body:"Milk\nCoffee\nFresh bread\nApples".into(),modified:NOW-86400},
    ];
    data.alarms = vec![Alarm {
        id: 601,
        label: "Good morning".into(),
        hour: 7,
        minute: 0,
        enabled: false,
        weekdays: vec![0, 1, 2, 3, 4],
        ..Alarm::default()
    }];
    data.photos = ["mountains", "coast", "forest", "city"]
        .into_iter()
        .enumerate()
        .map(|(i, source)| Photo {
            id: 701 + i as u64,
            name: match i {
                0 => "A new day",
                1 => "By the sea",
                2 => "The quiet path",
                _ => "City lights",
            }
            .into(),
            source: source.into(),
            favorite: i == 0,
        })
        .collect();
    data.notifications = vec![
        Notification {
            id: 801,
            app: AppId::Messaging,
            title: "Alex Morgan".into(),
            body: "Perfect, see you there ☺".into(),
            timestamp: NOW - 2400,
            route: Route::Conversation(201),
        },
        Notification {
            id: 802,
            app: AppId::Outlook,
            title: "Alex Morgan".into(),
            body: "Weekend plans".into(),
            timestamp: NOW - 3600,
            route: Route::Mail(301),
        },
    ];
    data.call_history = vec![
        CallRecord {
            id: 901,
            name: "Alex Morgan".into(),
            number: "+1 202 555 0101".into(),
            timestamp: NOW - 1800,
            incoming: true,
            missed: true,
        },
        CallRecord {
            id: 902,
            name: "Jamie Chen".into(),
            number: "+1 202 555 0102".into(),
            timestamp: NOW - 86400,
            incoming: false,
            missed: false,
        },
    ];
    data.bookmarks = vec![Bookmark {
        id: 951,
        title: "FerroUI".into(),
        url: "https://github.com/wieslawsoltes/FerroUI".into(),
    }];
    data
}
