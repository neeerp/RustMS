#include "UIFriendGroup.h"

#include "UINotice.h"

#include "../BuddyCommands.h"
#include "../KeyAction.h"
#include "../UI.h"
#include "../../Gameplay/Stage.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Audio/Audio.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The controls (FUN_0080f15b), placed by their top left.
		constexpr Point<int16_t> CLOSE_POS = Point<int16_t>(248, 6);
		constexpr Point<int16_t> OK_POS = Point<int16_t>(66, 328);
		constexpr Point<int16_t> CANCEL_POS = Point<int16_t>(144, 328);
		// The name's edit, 250 wide and 15 high at (9, 65), holds a group's
		// 16 characters; HeavenClient draws text 5 lower than v83.
		constexpr Rectangle<int16_t> NAME = Rectangle<int16_t>(11, 257, 60, 75);
		constexpr size_t GROUP_LIMIT = 16;
		// A checkbox control a row, 93 by 14, its box 2 right and 1 down in
		// it and the buddy's name after the box; the group is in the second
		// column (0x80f670).
		constexpr int16_t CHECK_X = 12;
		constexpr int16_t CHECK_Y = 137;
		constexpr int16_t CHECK_WIDTH = 93;
		constexpr int16_t CHECK_HEIGHT = 14;
		constexpr Point<int16_t> CHECK_BOX = Point<int16_t>(2, 1);
		constexpr int16_t CHECK_NAME_X = 30;
		constexpr int16_t GROUP_X = 113;
		constexpr int16_t ROW_TEXT_Y = 139;
		constexpr int16_t ROW = 20;
		constexpr Range<int16_t> SLIDER = Range<int16_t>(113, 277);
		constexpr int16_t SLIDER_X = 243;

		// How far down to draw Arial 12 whose top v83 puts at y: HC puts a
		// line's baseline a full line below where it's drawn.
		int16_t text_y(int16_t y)
		{
			static const int16_t line = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "|").height();

			return y + 11 - line;
		}
	}

	UIFriendGroup::UIFriendGroup(const std::string& g, const std::vector<Buddy>& buddies)
			: UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), group(g), renaming(!g.empty())
	{
		nl::node art = NxFiles::ui_classic["UIWindow.img"]["UserList"]["Friend"]["GroupMod"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		background = art["backgrnd"];
		checkbox[false] = basic["CheckBox"]["0"];
		checkbox[true] = basic["CheckBox"]["1"];

		dimension = Point<int16_t>(background.width(), background.height());
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(basic["BtClose"], CLOSE_POS);
		buttons[Buttons::OK] = std::make_unique<MapleButton>(art["BtOk"], OK_POS);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(art["BtCancel"], CANCEL_POS);

		// Every buddy, checked if already in the group; requests wait on
		// their cards instead.
		for (const Buddy& buddy : buddies)
		{
			if (buddy.pending())
				continue;

			bool member = renaming && buddy.group == group;
			picks.push_back({ buddy.name, buddy.group, member });
			names.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, buddy.name);
			groups.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, buddy.group);
		}

		// Default Group is taken even with nobody in it.
		taken = FriendGroups::names(buddies);
		taken.push_back(FriendGroups::DEFAULT);

		name = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, NAME, GROUP_LIMIT);
		name.change_text(group);
		name.set_enter_callback([&](std::string) { ok(); });
		name.set_key_callback(KeyAction::Id::ESCAPE, [&]() { close(); });
		name.set_state(Textfield::State::FOCUSED);

		slider = Slider(Slider::Type::CLASSIC, SLIDER, SLIDER_X, SHOWN, static_cast<int16_t>(picks.size()),
						[&](bool up) { scroll(top + (up ? -1 : 1)); });

		Sound(Sound::Name::DLGNOTICE).play();
	}

	void UIFriendGroup::draw(float alpha) const
	{
		background.draw(position);
		name.draw(position);

		for (size_t i = 0; i < SHOWN && top + i < picks.size(); i++)
		{
			size_t index = top + i;
			int16_t y = static_cast<int16_t>(CHECK_Y + ROW * i);
			int16_t row_text = text_y(static_cast<int16_t>(ROW_TEXT_Y + ROW * i));

			checkbox[picks[index].checked].draw(position + Point<int16_t>(CHECK_X, y) + CHECK_BOX);
			names[index].draw(position + Point<int16_t>(CHECK_NAME_X, row_text));
			groups[index].draw(position + Point<int16_t>(GROUP_X, row_text));
		}

		slider.draw(position);

		UIElement::draw(alpha);
	}

	void UIFriendGroup::update()
	{
		UIElement::update();

		name.update(position);
	}

	Cursor::State UIFriendGroup::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursor = cursorpos - position;

		if (Cursor::State state = slider.send_cursor(cursor, clicked); state != Cursor::State::IDLE)
			return state;

		Cursor::State field = name.send_cursor(cursorpos, clicked);

		if (field != Cursor::State::IDLE)
			return field;

		for (size_t i = 0; i < SHOWN && top + i < picks.size(); i++)
		{
			Rectangle<int16_t> control = Rectangle<int16_t>(CHECK_X, CHECK_X + CHECK_WIDTH,
					static_cast<int16_t>(CHECK_Y + ROW * i), static_cast<int16_t>(CHECK_Y + ROW * i + CHECK_HEIGHT));

			if (control.contains(cursor))
			{
				if (clicked)
				{
					picks[top + i].checked = !picks[top + i].checked;

					// IDLE, so holding the button doesn't toggle it back.
					return Cursor::State::IDLE;
				}

				return Cursor::State::CANCLICK;
			}
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIFriendGroup::send_scroll(double yoffset)
	{
		slider.send_scroll(yoffset);
	}

	void UIFriendGroup::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			ok();
	}

	UIElement::Type UIFriendGroup::get_type() const
	{
		return TYPE;
	}

	Button::State UIFriendGroup::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			ok();
		else
			close();

		return Button::State::NORMAL;
	}

	void UIFriendGroup::scroll(int16_t row)
	{
		int16_t last = std::max<int16_t>(0, static_cast<int16_t>(picks.size()) - SHOWN);
		top = std::clamp<int16_t>(row, 0, last);
		slider.setrows(top, SHOWN, static_cast<int16_t>(picks.size()));
	}

	void UIFriendGroup::ok()
	{
		std::string text = name.get_text();

		// v83 asks only of the 7 checkboxes in view.
		bool any_checked = false;

		for (size_t i = 0; i < SHOWN && top + i < picks.size(); i++)
			any_checked = any_checked || picks[top + i].checked;

		const char* refusal = nullptr;

		switch (FriendGroups::check_group(text, group, renaming, any_checked, taken))
		{
			case FriendGroups::Refusal::NO_NAME:
				refusal = "Please enter the group name.";
				break;
			case FriendGroups::Refusal::NO_FRIENDS:
				refusal = "You have not selected any friends.";
				break;
			case FriendGroups::Refusal::EXISTS:
				refusal = "This name already exists.";
				break;
			case FriendGroups::Refusal::NONE:
				break;
		}

		if (refusal)
		{
			// The notice takes the keys until it is closed.
			name.set_state(Textfield::State::DISABLED);
			UI::get().emplace<UIOk>(refusal, [this](bool) { name.set_state(Textfield::State::FOCUSED); });
			return;
		}

		std::vector<std::pair<std::string, std::string>> moves = FriendGroups::group_moves(picks, text);

		close();

		// Skip anyone deleted while the dialog was open.
		const BuddyList& buddies = Stage::get().get_player().get_buddies();

		for (const auto& [buddy, to] : moves)
			if (buddies.find_listed(buddy))
				BuddyCommands::add(buddy, to, false);
	}

	void UIFriendGroup::close()
	{
		name.set_state(Textfield::State::DISABLED);
		deactivate();
	}
}
