#include "UIWishlistInput.h"
#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Net/Packets/MarriagePackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The v83 window's layout (0x9d7822). Buttons put their top-left at
		// their position, whatever the canvas origin.
		constexpr Point<int16_t> ENTER_AT = Point<int16_t>(168, 56);
		constexpr Point<int16_t> OK_AT = Point<int16_t>(109, 279);
		constexpr Point<int16_t> DELETE_AT = Point<int16_t>(56, 279);
		constexpr Rectangle<int16_t> FIELD = Rectangle<int16_t>(13, 162, 57, 70);
		constexpr size_t FIELD_LIMIT = 16;

		// Ten rows 18 apart, each highlighted from x 7 and written from x 14.
		constexpr int16_t ROW_TOP = 86;
		constexpr int16_t ROW_PITCH = 18;
		constexpr int16_t ROW_LEFT = 7;
		constexpr int16_t ROW_WIDTH = 182;
		constexpr int16_t ROW_HEIGHT = 18;
		constexpr Point<int16_t> TEXT_AT = Point<int16_t>(14, 87);
	}

	UIWishlistInput::UIWishlistInput(bool female) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), selection(-1)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Wedding"]["wishList"]["input"];

		background = src[female ? "bride" : "groom"];
		selected_row = src["select"];
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
			(Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		auto button = [&](const char* name, Point<int16_t> at)
		{
			nl::node bt = src[name];

			return std::make_unique<MapleButton>(bt, at + Texture(bt["normal"]["0"]).get_origin());
		};

		buttons[Buttons::BT_ENTER] = button("BtEnter", ENTER_AT);
		buttons[Buttons::BT_OK] = button("BtOK", OK_AT);
		buttons[Buttons::BT_DELETE] = button("BtDelete", DELETE_AT);

		textfield = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, FIELD, FIELD_LIMIT);
		textfield.set_enter_callback([&](std::string) { enter(); });
		// Escape doesn't leave (0x9d8373).
		textfield.set_key_callback(KeyAction::Id::ESCAPE, []() {});
		textfield.set_state(Textfield::State::FOCUSED);
	}

	void UIWishlistInput::draw(float inter) const
	{
		// The art is centred on its origin.
		background.draw(position + background.get_origin());

		for (size_t i = 0; i < labels.size(); i++)
		{
			Point<int16_t> row = position + Point<int16_t>(0, ROW_PITCH * static_cast<int16_t>(i));

			if (static_cast<int16_t>(i) == selection)
				selected_row.draw(row + Point<int16_t>(ROW_LEFT, ROW_TOP));

			labels[i].draw(row + TEXT_AT);
		}

		textfield.draw(position);

		UIElement::draw(inter);
	}

	void UIWishlistInput::update()
	{
		UIElement::update();

		textfield.update(position);
	}

	Cursor::State UIWishlistInput::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State field = textfield.send_cursor(cursorpos, clicked);

		if (field != Cursor::State::IDLE)
			return field;

		if (clicked)
		{
			Point<int16_t> offset = cursorpos - position - Point<int16_t>(ROW_LEFT, ROW_TOP);

			if (offset.x() >= 0 && offset.x() < ROW_WIDTH && offset.y() >= 0 && offset.y() % ROW_PITCH < ROW_HEIGHT)
			{
				int16_t row = offset.y() / ROW_PITCH;

				if (row < static_cast<int16_t>(wishes.size()))
				{
					selection = row;
					// The field keeps the keys.
					textfield.set_state(Textfield::State::FOCUSED);

					return Cursor::State::CLICKING;
				}
			}
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIWishlistInput::send_key(int32_t keycode, bool pressed, bool)
	{
		if (pressed && keycode == KeyAction::Id::RETURN)
			enter();
	}

	UIElement::Type UIWishlistInput::get_type() const
	{
		return TYPE;
	}

	Button::State UIWishlistInput::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::BT_ENTER:
				enter();
				break;
			case Buttons::BT_OK:
				submit();
				break;
			case Buttons::BT_DELETE:
				remove();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIWishlistInput::enter()
	{
		std::string wish = textfield.get_text();

		if (wish.empty())
			return;

		textfield.change_text("");

		if (wishes.size() >= MAX_WISHES)
		{
			// The notice takes the keys while it is up.
			textfield.set_state(Textfield::State::DISABLED);
			UI::get().emplace<UIOk>("You have exceeded the maximum number of requests.", [this](bool) { refocus(); });

			return;
		}

		wishes.push_back(wish);
		labels.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, wish);
	}

	void UIWishlistInput::remove()
	{
		if (selection < 0 || selection >= static_cast<int16_t>(wishes.size()))
			return;

		wishes.erase(wishes.begin() + selection);
		labels.erase(labels.begin() + selection);
		selection = -1;
	}

	void UIWishlistInput::submit()
	{
		// The question takes the keys while it is up.
		textfield.set_state(Textfield::State::DISABLED);

		UI::get().emplace<UIYesNo>(
			"This is your last chance to make any\\nchanges. Are you sure that you want to\\nsubmit this list?",
			[this](bool yes)
			{
				if (!yes)
				{
					refocus();
					return;
				}

				if (!is_active())
					return;

				WishlistPacket(wishes).dispatch();
				textfield.set_state(Textfield::State::DISABLED);
				deactivate();
			});
	}

	void UIWishlistInput::refocus()
	{
		if (is_active())
			textfield.set_state(Textfield::State::FOCUSED);
	}
}
