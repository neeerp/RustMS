#include "UIAvatarSelect.h"

#include "UINpcTalk.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/NpcInteractionPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NpcTextMarkup.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The window is the background's size, centred on the screen. The rest is from its top left
		// (FUN_009a92c9, FUN_009aa708, and FUN_009a4f1d for the shadow).
		constexpr int16_t WIDTH = 419, HEIGHT = 306;
		const Point<int16_t> PREV_AT(15, 195);
		const Point<int16_t> NEXT_AT(341, 195);
		const Point<int16_t> OK_AT(314, 281);
		const Point<int16_t> CANCEL_AT(363, 281);
		const Point<int16_t> EXIT_AT(9, 281);
		const Point<int16_t> OFF_AT(172, 281);
		const Point<int16_t> FEET(207, 245);
		const Point<int16_t> SHADOW_AT(174, 239);
		const Point<int16_t> NAME_TAG_AT(123, 246);
		// The name is centred 87 across the tag, 2 down.
		const Point<int16_t> NAME_AT(123 + 87, 246 + 2);
		// The NPC's middle is 45 across, 78 down.
		constexpr int16_t SPEAKER_X = 45, SPEAKER_Y = 78;
		// The question's box (FUN_009acc20): it starts 110 across and is centred on 100 rows from 28 down
		// when it fits, and FUN_009a3e9f draws the text 10 px right and 2 down of that. The box is 280 wide;
		// footage breaks lines as if 10 px were kept clear on each side.
		constexpr int16_t PROMPT_X = 110, PROMPT_TOP = 28, PROMPT_ROWS = 100;
		constexpr int16_t PROMPT_INSET_X = 10, PROMPT_INSET_Y = 2;
		constexpr uint16_t PROMPT_WIDTH = 280 - 2 * PROMPT_INSET_X;
		// v83 draws a line's cap top about 3 below where it is drawn, we 9, so text goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
		// A question taller than the box starts 6 higher and 2 further left, with a scroll bar (skin 3, VScr4)
		// 280 - 3 right of that, 2 below its top and 103 tall (FUN_009acc20, 0x9a63f2). It has one step more
		// than the eighths of the overflow, two at least, and each step moves the text 8 up (0x9ac4e2).
		constexpr int16_t SCROLL_RISE = 6, SCROLL_SHIFT = 2, SCROLL_STEP = 8;
		constexpr int16_t SLIDER_X = PROMPT_X - SCROLL_SHIFT + 280 - 3, SLIDER_Y = PROMPT_TOP - SCROLL_RISE + 2;
		constexpr int16_t SLIDER_HEIGHT = PROMPT_ROWS + 3, SLIDER_ARROW = 13;
	}

	UIAvatarSelect::UIAvatarSelect(int32_t npcid, int8_t mt, const std::string &text, std::vector<int32_t> s)
			: UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, HEIGHT)), msgtype(mt), styles(std::move(s)),
			  shown(0), bare(false), prompt_length(0), typed(0), timestep(0), scrolls(false), scroll(0)
	{
		switch (styles.front() / 10000)
		{
			case 2:
				kind = Kind::FACE;
				break;
			case 3:
				kind = Kind::HAIR;
				break;
			default:
				kind = Kind::SKIN;
				break;
		}

		nl::node avatar = NxFiles::ui_classic["UIWindow.img"]["UtilDlgEx_Avatar"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		sprites.emplace_back(avatar["backgrnd"]);
		shadow = avatar["shadow"];
		name_tag = avatar["nameTag"];
		speaker = NxHelper::Npc::get_npc_node(npcid)["stand"]["0"];

		buttons[Buttons::PREV] = std::make_unique<MapleButton>(avatar["BtPrev"], PREV_AT);
		buttons[Buttons::NEXT] = std::make_unique<MapleButton>(avatar["BtNext"], NEXT_AT);
		buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_AT);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], CANCEL_AT);
		buttons[Buttons::EXIT] = std::make_unique<MapleButton>(avatar["BtExit"], EXIT_AT);
		buttons[Buttons::OFF] = std::make_unique<MapleButton>(avatar["BtOff"], OFF_AT);
		buttons[Buttons::ON] = std::make_unique<MapleButton>(avatar["BtOn"], OFF_AT);
		buttons[Buttons::ON]->set_active(false);

		// The question is drawn as the dialogue's is, its images too
		// (FUN_009a3e9f).
		std::vector<NpcTextMarkup::InlineImage> tags;
		std::string formatted = NpcTextMarkup::expand(text, UINpcTalk::markup_lookups(), &tags);
		std::vector<Point<int16_t>> boxes;

		for (const auto &tag : tags)
		{
			prompt_images.push_back(UINpcTalk::inline_image(tag));
			prompt_images.back().box = boxes.size();
			boxes.push_back(prompt_images.back().size);
		}

		prompt = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::DARKGREY, formatted, PROMPT_WIDTH);
		prompt_length = formatted.size();
		prompt_instant = UINpcTalk::instant_ranges(formatted);
		prompt.set_boxes(boxes);

		if (int16_t overflow = prompt.height() - PROMPT_ROWS; overflow > 0)
		{
			int16_t steps = overflow / SCROLL_STEP;

			scrolls = true;
			slider = Slider(Slider::Type::CLASSIC, Range<int16_t>(SLIDER_Y, SLIDER_Y + SLIDER_HEIGHT - SLIDER_ARROW),
							SLIDER_X, 1, steps + (steps < 1 ? 2 : 1),
							[this](bool upwards) { scroll += upwards ? -1 : 1; });
		}

		name = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE);

		show(0);
		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIAvatarSelect::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		speaker.draw(position + Point<int16_t>(SPEAKER_X, SPEAKER_Y + speaker.height() / 2));

		int16_t top = scrolls ? PROMPT_TOP - SCROLL_RISE : PROMPT_TOP + (PROMPT_ROWS - prompt.height() - 6) / 2;
		int16_t left = scrolls ? PROMPT_X - SCROLL_SHIFT : PROMPT_X;

		Point<int16_t> origin = position + Point<int16_t>(left + PROMPT_INSET_X,
														  top + PROMPT_INSET_Y - TEXT_RAISE - scroll * SCROLL_STEP);
		// A scrolling question shows only what is in its box.
		Range<int16_t> clip = scrolls ? Range<int16_t>(position.y() + top, position.y() + top + PROMPT_ROWS + SCROLL_RISE)
									  : Range<int16_t>(0, 0);

		for (const auto &image : prompt_images)
		{
			Point<int16_t> box = origin + prompt.box_position(image.box);

			if (!scrolls || (box.y() >= clip.first() && box.y() + image.size.y() <= clip.second()))
				image.texture.draw(box + image.anchor);
		}

		// The question types out; its images show at once, as the dialogue's do.
		prompt.draw(origin, clip, typed, prompt_instant);

		if (scrolls)
			slider.draw(position);

		shadow.draw(position + SHADOW_AT);
		look.draw(DrawArgument(position + FEET), alpha);

		// Skins have no name.
		if (kind != Kind::SKIN)
		{
			name_tag.draw(position + NAME_TAG_AT);
			name.draw(position + NAME_AT - Point<int16_t>(0, TEXT_RAISE));
		}

		UIElement::draw_buttons(alpha);
	}

	void UIAvatarSelect::update()
	{
		UIElement::update();

		look.update(Constants::TIMESTEP);

		// The avatar dialogue is the same one, never compact (0x747206), so
		// its question types out too.
		if (typed < prompt_length)
			UINpcTalk::type_on(typed, timestep, prompt_length, prompt_instant);
	}

	void UIAvatarSelect::update_screen(int16_t new_width, int16_t new_height)
	{
		int16_t x = std::clamp<int16_t>(new_width / 2 - WIDTH / 2, 0, std::max<int16_t>(new_width - WIDTH, 0));
		int16_t y = std::clamp<int16_t>(new_height / 2 - HEIGHT / 2, 0, std::max<int16_t>(new_height - HEIGHT, 0));

		position = Point<int16_t>(x, y);
	}

	void UIAvatarSelect::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		// Escape is Cancel and Enter OK; the arrows do nothing here.
		if (escape)
			answer(false);
		else if (keycode == KeyAction::Id::RETURN)
			answer(true);
	}

	Cursor::State UIAvatarSelect::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (scrolls && slider.isenabled())
			if (Cursor::State state = slider.send_cursor(cursorpos - position, clicked))
				return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIAvatarSelect::send_scroll(double yoffset)
	{
		if (scrolls && slider.isenabled())
			slider.send_scroll(yoffset);
	}

	void UIAvatarSelect::remove_cursor()
	{
		UIElement::remove_cursor();

		if (scrolls)
			slider.remove_cursor();
	}

	UIElement::Type UIAvatarSelect::get_type() const
	{
		return TYPE;
	}

	Button::State UIAvatarSelect::button_pressed(uint16_t buttonid)
	{
		size_t count = styles.size();

		switch (buttonid)
		{
			case Buttons::OK:
				answer(true);
				break;
			case Buttons::CANCEL:
			case Buttons::EXIT:
				answer(false);
				break;
			case Buttons::PREV:
				show((shown + count - 1) % count);
				break;
			case Buttons::NEXT:
				show((shown + 1) % count);
				break;
			case Buttons::OFF:
			case Buttons::ON:
				bare = buttonid == Buttons::OFF;
				buttons[Buttons::OFF]->set_active(!bare);
				buttons[Buttons::ON]->set_active(bare);
				show(shown);
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UIAvatarSelect::show(size_t index)
	{
		shown = index;

		look = Stage::get().get_player().get_look();

		if (bare)
			for (auto slot : EquipSlot::values)
				look.remove_equip(slot);

		// Standing still, facing left, off any mount.
		look.reset();
		look.set_vehicle(0);
		look.set_direction(false);
		look.set_alerted(0);

		int32_t style = styles[index];

		switch (kind)
		{
			case Kind::FACE:
				look.set_face(style);
				name.change_text(look.get_face()->get_name());
				break;
			case Kind::HAIR:
				look.set_hair(style);
				name.change_text(look.get_hair()->get_name());
				break;
			case Kind::SKIN:
				look.set_body(style);
				break;
		}

		// Clears the alert just set, which the player may have had.
		look.update(Constants::TIMESTEP);
	}

	void UIAvatarSelect::answer(bool ok)
	{
		deactivate();

		if (ok)
			NpcTalkStylePacket(msgtype, static_cast<uint8_t>(shown)).dispatch();
		else
			NpcTalkMorePacket(msgtype, 0).dispatch();
	}
}
