use dioxus::prelude::*;

/// Icon element
pub trait IconLike: Fn(&str, &str) -> Element {}

impl<T: Fn(&str, &str) -> Element> IconLike for T {}

#[component]
pub fn Icon(icon: IconType, size: String) -> Element {
	rsx!(IconImpl {
		icon: icon.file(),
		width: size.clone(),
		height: size,
	})
}

#[component]
fn IconImpl(icon: &'static str, width: String, height: String) -> Element {
	rsx! {
		svg {
			width: width,
			height: height,
			fill: "currentColor",
			view_box: "0 0 16 16",
			xmlns: "http://www.w3.org/2000/svg",
			dangerous_inner_html: "{icon}",
		}
	}
}

macro_rules! icons {
	($($name:ident $id:literal;)*) => {
		/// Different types of icons
		#[derive(Clone, Copy, PartialEq, Eq)]
		pub enum IconType {
			$(
				$name,
			)*
		}

		impl IconType {
			/// Gets the svg data associated with this icon
			pub fn file(&self) -> &'static str {
				match self {
					$(
						Self::$name => include_str!(concat!("../../assets/icons/", $id, ".svg")),
					)*
				}
			}
		}
	};
}

icons! {
	AngleDown "angle_down";
	AngleLeft "angle_left";
	AngleRight "angle_right";
	Animal "animal";
	ArrowLeft "arrow_left";
	ArrowRight "arrow_right";
	Audio "audio";
	Book "book";
	Box "box";
	Building "building";
	Burger "burger";
	Bus "bus";
	Check "check";
	Connections "connections";
	Controller "controller";
	Copy "copy";
	Couch "couch";
	Cross "cross";
	CurlyBraces "curly_braces";
	Cycle "cycle";
	Delete "delete";
	Diagram "diagram";
	Dice "dice";
	Download "download";
	Dumbbell "dumbbell";
	Edit "edit";
	Elipsis "elipsis";
	Error "error";
	Folder "folder";
	Font "font";
	Fork "fork";
	Fullscreen "fullscreen";
	Gear "gear";
	Globe "globe";
	Graph "graph";
	Hashtag "hashtag";
	Heart "heart";
	Helmet "helmet";
	Home "home";
	Honeycomb "honeycomb";
	Info "info";
	Jigsaw "jigsaw";
	Key "key";
	Language "language";
	Lightning "lightning";
	LinkBroken "link_broken";
	Link "link";
	LockOpen "lock_open";
	Lock "lock";
	Logo "logo";
	MapPin "map_pin";
	Menu "menu";
	Microphone "microphone";
	Minecraft "minecraft";
	Moon "moon";
	MultipleUsers "multiple_users";
	Notification "notification";
	Palette "palette";
	Picture "picture";
	Pin "pin";
	Play "play";
	Plus "plus";
	Popout "popout";
	Properties "properties";
	Refresh "refresh";
	Scroll "scroll";
	Search "search";
	Server "server";
	Speed "speed";
	Spinner "spinner";
	Star "star";
	Stop "stop";
	Sun "sun";
	Sword "sword";
	Tag "tag";
	Text "text";
	Trash "trash";
	Upload "upload";
	User "user";
	Warning "warning";
	Window "window";
}
