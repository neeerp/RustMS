#include "UISlideMenu.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Net/Packets/NpcInteractionPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The window is the background's size, centred on the screen. The rest is from its top left
		// (FUN_0076b9fc, FUN_0076be7a, FUN_0076c389).
		constexpr int16_t WIDTH = 235, HEIGHT = 165;
		const Point<int16_t> LEFT_AT(11, 67);
		const Point<int16_t> RIGHT_AT(200, 67);
		const Point<int16_t> MOVE_AT(69, 134);
		const Point<int16_t> CANCEL_AT(128, 134);
		const Point<int16_t> BEFORE_AT(45, 58);
		const Point<int16_t> MIDDLE_AT(97, 58);
		const Point<int16_t> AFTER_AT(149, 58);
		// The middle choice's label is centred 117 across, 107 down.
		const Point<int16_t> LABEL_AT(117, 107);
		// v83 draws a line's cap top about 3 below where it is drawn, we 9, so text goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
		constexpr int8_t MSGTYPE = 14;
	}

	UISlideMenu::UISlideMenu(int32_t menutype, slide_menu::Menu m)
			: UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, HEIGHT)), menu(std::move(m)), shown(menu.start)
	{
		src = NxFiles::ui_classic["UIWindow.img"]["SlideMenu"][std::to_string(menutype)];

		sprites.emplace_back(src["backgrd"]);
		// Over the icons, the middle one framed.
		cover = src["Cover"];
		choice = src["Choice"];

		buttons[Buttons::LEFT] = std::make_unique<MapleButton>(src["BtArrow"]["left"], LEFT_AT);
		buttons[Buttons::RIGHT] = std::make_unique<MapleButton>(src["BtArrow"]["right"], RIGHT_AT);
		buttons[Buttons::MOVE] = std::make_unique<MapleButton>(src["BtMove"], MOVE_AT);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(src["BtCancle"], CANCEL_AT);

		label = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE);

		show(shown);
		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UISlideMenu::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		// v83 draws on the window's own layer, so a long label is cut off at its edges.
		label.draw_between(position + LABEL_AT - Point<int16_t>(0, TEXT_RAISE),
						   Range<int16_t>(position.x(), position.x() + WIDTH));

		UIElement::draw_buttons(alpha);

		cover.draw(position + BEFORE_AT);
		choice.draw(position + MIDDLE_AT);
	}

	void UISlideMenu::update_screen(int16_t new_width, int16_t new_height)
	{
		int16_t x = std::clamp<int16_t>(new_width / 2 - WIDTH / 2, 0, std::max<int16_t>(new_width - WIDTH, 0));
		int16_t y = std::clamp<int16_t>(new_height / 2 - HEIGHT / 2, 0, std::max<int16_t>(new_height - HEIGHT, 0));

		position = Point<int16_t>(x, y);
	}

	void UISlideMenu::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		// v83 FUN_0076c5df: Escape is Cancel, Enter Move, and the arrows slide when they can.
		if (escape)
			answer(false);
		else if (keycode == KeyAction::Id::RETURN && !menu.empty)
			answer(true);
		else if (keycode == KeyAction::Id::LEFT && shown > 0)
			show(shown - 1);
		else if (keycode == KeyAction::Id::RIGHT && shown + 1 < menu.entries.size())
			show(shown + 1);
	}

	UIElement::Type UISlideMenu::get_type() const
	{
		return TYPE;
	}

	Button::State UISlideMenu::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::MOVE:
			case Buttons::MIDDLE:
				answer(true);
				break;
			case Buttons::CANCEL:
				answer(false);
				break;
			case Buttons::LEFT:
				show(shown - 1);
				break;
			case Buttons::RIGHT:
				show(shown + 1);
				break;
			default:
				break;
		}

		return buttons[buttonid]->get_state();
	}

	void UISlideMenu::show(size_t index)
	{
		shown = index;

		size_t count = menu.entries.size();
		bool before = !menu.empty && shown > 0;
		bool after = !menu.empty && shown + 1 < count;

		buttons[Buttons::LEFT]->set_state(before ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[Buttons::RIGHT]->set_state(after ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[Buttons::MOVE]->set_state(menu.empty ? Button::State::DISABLED : Button::State::NORMAL);

		buttons.erase(Buttons::BEFORE);
		buttons.erase(Buttons::MIDDLE);
		buttons.erase(Buttons::AFTER);

		auto icon = [this](size_t entry)
		{
			return src["BtMain"][std::to_string(menu.entries[entry].id)];
		};

		// The icons either side only show what comes next: v83 disables them and draws them in grey
		// (76c2ab).
		if (!menu.empty)
		{
			buttons[Buttons::MIDDLE] = std::make_unique<MapleButton>(icon(shown), MIDDLE_AT);

			auto beside = [&](size_t entry, Point<int16_t> at)
			{
				auto button = std::make_unique<MapleButton>(icon(entry), at);
				button->set_state(Button::State::DISABLED);
				button->grey_when_disabled();

				return button;
			};

			if (before)
				buttons[Buttons::BEFORE] = beside(shown - 1, BEFORE_AT);

			if (after)
				buttons[Buttons::AFTER] = beside(shown + 1, AFTER_AT);
		}

		label.change_text(shown < count ? menu.entries[shown].label : "");
	}

	void UISlideMenu::answer(bool ok)
	{
		deactivate();

		if (ok)
			NpcTalkMorePacket(MSGTYPE, 1, menu.entries[shown].id).dispatch();
		else
			NpcTalkMorePacket(MSGTYPE, 0).dispatch();
	}
}
