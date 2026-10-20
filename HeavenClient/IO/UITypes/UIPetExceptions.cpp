#include "UIPetExceptions.h"

#include "UIPetItemSearch.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Character/Pets.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// Laid out by FUN_008fc61b and FUN_008fca75, against the window's
		// top-left corner.
		const Point<int16_t> HIDE = { 142, 5 };
		const Point<int16_t> MESO = { 10, 124 };
		const Point<int16_t> REGISTER = { 80, 124 };
		const Point<int16_t> DELETE = { 124, 124 };
		// The scrollbar is 90 tall from (140, 25); ours puts its lower
		// arrow's top at the range's end.
		constexpr int16_t SLIDER_X = 140;
		constexpr Range<int16_t> SLIDER = Range<int16_t>(25, 25 + 90 - 13);
		// Six rows of names, each lit by a box behind it when selected or
		// under the cursor.
		constexpr int16_t ROWS = 6, ROW_TOP = 30, ROW_STEP = 14;
		constexpr int16_t NAME_X = 15, FILL_X = 10, FILL_WIDTH = 138, FILL_HEIGHT = 13, HIT_WIDTH = 138;
		constexpr Color SELECTED = Color(0x5B7B9EFF);
		constexpr Color HOVERED = Color(0xBBC9D9FF);

		// How far down to draw Arial 12 whose top v83 puts at y: HC puts a
		// line's baseline a full line below where it's drawn.
		int16_t text_y(int16_t y)
		{
			static const int16_t line = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "|").height();

			return y + 11 - line;
		}

		std::string item_name(int32_t id)
		{
			return ItemData::get(id).get_name();
		}
	}

	UIPetExceptions::UIPetExceptions(uint8_t p, Point<int16_t> pos) : UIElement(pos, Point<int16_t>()), pet(p)
	{
		nl::node UserInfo = NxFiles::ui_classic["UIWindow.img"]["UserInfo"];

		background = UserInfo["backgrnd6"];
		dimension = background.get_dimensions();

		buttons[Buttons::BT_HIDE] = std::make_unique<MapleButton>(UserInfo["BtExceptionHide"], HIDE);
		buttons[Buttons::BT_MESO] = std::make_unique<MapleButton>(UserInfo["BtMeso"], MESO);
		buttons[Buttons::BT_REGISTER] = std::make_unique<MapleButton>(UserInfo["BtRegister"], REGISTER);
		buttons[Buttons::BT_DELETE] = std::make_unique<MapleButton>(UserInfo["BtDelete"], DELETE);

		slider = Slider(Slider::Type::CLASSIC, SLIDER, SLIDER_X, ROWS, 0, [&](bool upwards)
		{
			offset += upwards ? -1 : 1;
		});

		refresh();
	}

	void UIPetExceptions::draw(float alpha) const
	{
		background.draw(position);

		GraphicsGL &graphics = GraphicsGL::get();

		for (int16_t r = 0; r < ROWS && offset + r < static_cast<int16_t>(names.size()); r++)
		{
			int16_t i = offset + r;
			int16_t top = ROW_TOP + ROW_STEP * r;

			if (i == selected || i == hovered)
			{
				Color fill = i == selected ? SELECTED : HOVERED;
				graphics.drawrectangle(position.x() + FILL_X, position.y() + top, FILL_WIDTH, FILL_HEIGHT, fill.r(),
									   fill.g(), fill.b(), fill.a());
			}

			names[i].draw(position + Point<int16_t>(NAME_X, text_y(top)));
		}

		slider.draw(position);

		UIElement::draw_buttons(alpha);
	}

	void UIPetExceptions::update()
	{
		UIElement::update();

		Optional<PetLook> look = Stage::get().get_player().get_pet(pet);

		if (look && look->get_ignored() != shown)
			refresh();
	}

	Cursor::State UIPetExceptions::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		bool released = !clicked && mouse_down;
		mouse_down = clicked;

		Cursor::State state = slider.send_cursor(cursorpos - position, clicked);

		if (state != Cursor::State::IDLE)
			return state;

		// A row lights under the cursor, and is selected as the button comes
		// up on it, or none is off the rows (FUN_008fce6a, FUN_008fcea0).
		hovered = row_at(cursorpos);

		if (released && !is_holding_button())
			selected = hovered;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIPetExceptions::remove_cursor()
	{
		UIElement::remove_cursor();

		slider.remove_cursor();
		hovered = -1;
		mouse_down = false;
	}

	void UIPetExceptions::send_scroll(double yoffset)
	{
		slider.send_scroll(yoffset);
	}

	UIElement::Type UIPetExceptions::get_type() const
	{
		return UIElement::Type::CHARINFO;
	}

	void UIPetExceptions::set_position(Point<int16_t> pos)
	{
		position = pos;
	}

	uint8_t UIPetExceptions::get_pet() const
	{
		return pet;
	}

	void UIPetExceptions::add(int32_t itemid)
	{
		Optional<PetLook> look = Stage::get().get_player().get_pet(pet);

		if (!look)
			return;

		std::vector<int32_t> ids = look->get_ignored();
		std::optional<size_t> slot = pets::ignored_slot(ids, itemid, item_name);

		if (!slot)
			return;

		ids.insert(ids.begin() + *slot, itemid);

		if (selected >= static_cast<int16_t>(*slot))
			selected++;

		look->set_ignored(ids);
		refresh();

		// A full list without mesos can't take them too.
		if (ids.size() == pets::MAX_IGNORED
			&& std::find(ids.begin(), ids.end(), pets::IGNORED_MESOS) == ids.end())
			buttons[Buttons::BT_MESO]->set_state(Button::State::DISABLED);

		PetExcludeItemsPacket(look->get_serial(), ids).dispatch();
	}

	void UIPetExceptions::remove(int32_t itemid)
	{
		Optional<PetLook> look = Stage::get().get_player().get_pet(pet);

		if (!look)
			return;

		std::vector<int32_t> ids = look->get_ignored();
		auto iter = std::find(ids.begin(), ids.end(), itemid);

		if (iter == ids.end())
			return;

		auto index = static_cast<int16_t>(iter - ids.begin());
		ids.erase(iter);
		look->set_ignored(ids);
		refresh();

		if (buttons[Buttons::BT_MESO]->get_state() == Button::State::DISABLED)
			buttons[Buttons::BT_MESO]->set_state(Button::State::NORMAL);

		if (selected == index)
			selected = -1;
		else if (selected > index)
			selected--;

		PetExcludeItemsPacket(look->get_serial(), ids).dispatch();
	}

	void UIPetExceptions::close()
	{
		if (auto search = UI::get().get_element<UIPetItemSearch>())
			search->close();

		if (Optional<PetLook> look = Stage::get().get_player().get_pet(pet))
			PetExcludeItemsPacket(look->get_serial(), look->get_ignored()).dispatch();
	}

	Button::State UIPetExceptions::button_pressed(uint16_t buttonid)
	{
		Optional<PetLook> look = Stage::get().get_player().get_pet(pet);

		// The hide button closes the window, as does any once the pet is
		// gone: the character info sees it inactive.
		if (!look || buttonid == Buttons::BT_HIDE)
		{
			deactivate();

			return Button::State::NORMAL;
		}

		const std::vector<int32_t> &ids = look->get_ignored();

		switch (buttonid)
		{
			case Buttons::BT_MESO:
				if (std::find(ids.begin(), ids.end(), pets::IGNORED_MESOS) != ids.end())
					remove(pets::IGNORED_MESOS);
				else
					add(pets::IGNORED_MESOS);

				break;
			case Buttons::BT_REGISTER:
			{
				auto search = UI::get().get_element<UIPetItemSearch>();

				if (!search || !search->is_active())
					UI::get().emplace<UIPetItemSearch>(position);

				break;
			}
			case Buttons::BT_DELETE:
				if (selected >= 0 && selected < static_cast<int16_t>(ids.size()))
					remove(ids[selected]);

				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UIPetExceptions::refresh()
	{
		Optional<PetLook> look = Stage::get().get_player().get_pet(pet);
		shown = look ? look->get_ignored() : std::vector<int32_t>();
		names.clear();

		for (int32_t id : shown)
			names.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
							   pets::ignored_name(id, item_name));

		auto count = static_cast<int16_t>(shown.size());
		offset = std::clamp<int16_t>(offset, 0, std::max<int16_t>(count - ROWS, 0));
		slider.setrows(offset, ROWS, count);
	}

	int16_t UIPetExceptions::row_at(Point<int16_t> cursorpos) const
	{
		Point<int16_t> pos = cursorpos - position;

		for (int16_t r = 0; r < ROWS && offset + r < static_cast<int16_t>(shown.size()); r++)
		{
			int16_t top = ROW_TOP + ROW_STEP * r;

			if (Rectangle<int16_t>(0, HIT_WIDTH, top, top + ROW_STEP).contains(pos))
				return offset + r;
		}

		return -1;
	}
}
