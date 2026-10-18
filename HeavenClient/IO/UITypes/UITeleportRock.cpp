#include "UITeleportRock.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Character/TeleportRock.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <cstdlib>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// FUN_008392c1 and FUN_008398d5, from the window's top left.
		constexpr int16_t ROWS = 5;
		constexpr int16_t ROW_PITCH = 18;
		constexpr Point<int16_t> ROW_BAR = Point<int16_t>(9, 84);
		constexpr Point<int16_t> ROW_TEXT = Point<int16_t>(15, 87);
		constexpr size_t ROW_WIDTH = 158;
		// A row is hit 150 wide and 15 tall, inside its bar.
		constexpr int16_t HIT_RIGHT = 159;
		constexpr int16_t HIT_HEIGHT = 15;
		// With a row selected, a click here deselects it.
		constexpr int16_t DESELECT_TOP = 191;
		constexpr int16_t DESELECT_BOTTOM = 279;
		constexpr int16_t COUNTER_RIGHT = 165;
		constexpr int16_t COUNTER_Y = 38;
		// v83 draws a line's cap top about 3 below where it is drawn, we 9,
		// so all text here goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
		constexpr Rectangle<int16_t> FIELD = Rectangle<int16_t>(34, 154, 250, 263);
		constexpr size_t NAME_LIMIT = 12;
		// The VIP rock's ten maps scroll through the five rows.
		constexpr int16_t SLIDER_X = 155;
		constexpr Range<int16_t> SLIDER = Range<int16_t>(80, 165);

		// The map's name, under the map it borrows its data from (FUN_005cf792).
		std::string map_name(int32_t map_id)
		{
			std::string link = NxHelper::Map::get_map_node_name(map_id)["info"]["link"];

			if (!link.empty())
				map_id = std::atoi(link.c_str());

			return NxHelper::Map::get_map_info_by_id(map_id).name;
		}
	}

	UITeleportRock::UITeleportRock(bool v, std::function<void(const std::string &, int32_t)> om)
			: UIElement(Point<int16_t>(), Point<int16_t>()), vip(v), onmove(std::move(om)), counter_width(0), first(0),
			  selected(-1), held(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Teleport"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);
		select = src["select"];

		buttons[Buttons::REGISTER] = std::make_unique<MapleButton>(src["BtRegister"], Point<int16_t>(6, 285));
		buttons[Buttons::DELETE] = std::make_unique<MapleButton>(src["BtDelete"], Point<int16_t>(56, 285));
		buttons[Buttons::MOVE] = std::make_unique<MapleButton>(src["BtMove"], Point<int16_t>(120, 285));
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"],
																Point<int16_t>(156, 6));

		// Until something is chosen.
		buttons[Buttons::DELETE]->set_state(Button::State::DISABLED);
		buttons[Buttons::MOVE]->set_state(Button::State::DISABLED);

		namefield = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, FIELD, NAME_LIMIT);

		namefield.set_enter_callback(
				[&](std::string)
				{
					if (buttons[Buttons::MOVE]->get_state() != Button::State::DISABLED)
						move();
				}
		);

		namefield.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					close();
				}
		);

		if (vip)
			slider = Slider(
					Slider::Type::CLASSIC, SLIDER, SLIDER_X, ROWS, static_cast<int16_t>(TeleportRock::slots(true)),
					[&](bool upwards)
					{
						first += upwards ? -1 : 1;
					}
			);

		// Centred, with no title bar to drag it by.
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		refresh();

		namefield.set_state(Textfield::State::FOCUSED);
	}

	void UITeleportRock::draw(float alpha) const
	{
		UIElement::draw(alpha);

		namefield.draw(position - Point<int16_t>(0, TEXT_RAISE));

		if (vip)
			slider.draw(position);

		counter.draw(position + Point<int16_t>(COUNTER_RIGHT - counter_width, COUNTER_Y - TEXT_RAISE));

		for (int16_t r = 0; r < ROWS; r++)
		{
			int16_t i = first + r;

			if (i >= static_cast<int16_t>(maps.size()) || maps[i] == TeleportRock::EMPTY)
				continue;

			Point<int16_t> offset = position + Point<int16_t>(0, ROW_PITCH * r);

			if (i == selected)
				select.draw(offset + ROW_BAR);

			const Row &row = rows[i];
			(i == selected ? row.selected : row.normal).draw(offset + ROW_TEXT - Point<int16_t>(0, TEXT_RAISE));
		}
	}

	void UITeleportRock::update()
	{
		UIElement::update();

		namefield.update(position);

		// FUN_00839f10.
		if (namefield.get_state() == Textfield::State::FOCUSED)
			selected = -1;

		auto enable = [this](Buttons id, bool enabled)
		{
			Button &button = *buttons[id];

			if (!enabled)
				button.set_state(Button::State::DISABLED);
			else if (button.get_state() == Button::State::DISABLED)
				button.set_state(Button::State::NORMAL);
		};

		enable(Buttons::MOVE, selected >= 0 || !namefield.empty());
		enable(Buttons::DELETE, selected >= 0);
	}

	void UITeleportRock::remove_cursor()
	{
		UIElement::remove_cursor();

		held = false;

		if (vip)
			slider.remove_cursor();
	}

	Cursor::State UITeleportRock::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// The rows answer only to the button going down.
		bool press = clicked && !held;
		held = clicked;

		Point<int16_t> cursor = cursorpos - position;

		if (vip)
		{
			Cursor::State state = slider.send_cursor(cursor, clicked);

			if (state != Cursor::State::IDLE)
				return state;
		}

		if (press && select_at(cursor))
			return Cursor::State::CLICKING;

		Cursor::State state = namefield.send_cursor(cursorpos, clicked);

		if (state != Cursor::State::IDLE)
			return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UITeleportRock::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		// FUN_004edd21: Enter is Move, if it may be pressed, and Escape is Close.
		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN && buttons[Buttons::MOVE]->get_state() != Button::State::DISABLED)
			move();
	}

	UIElement::Type UITeleportRock::get_type() const
	{
		return TYPE;
	}

	void UITeleportRock::refresh()
	{
		maps = Stage::get().get_player().get_teleport_rock().get_maps(vip);
		rows.clear();

		for (int32_t map_id : maps)
		{
			std::string name = map_id == TeleportRock::EMPTY ? "" : map_name(map_id);

			Text normal = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, name);
			string_format::format_with_ellipsis(normal, ROW_WIDTH);

			rows.push_back({ normal, Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, normal.get_text()) });
		}

		// Measured in the rows' font, but drawn smaller.
		std::string count = "[" + std::to_string(TeleportRock::used(maps)) + "/" + std::to_string(maps.size()) + "]";
		counter = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, count);
		counter_width = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, count).width();
	}

	Button::State UITeleportRock::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::REGISTER:
				keep();
				break;
			case Buttons::DELETE:
				drop();
				break;
			case Buttons::MOVE:
				move();
				break;
			case Buttons::CLOSE:
				close();
				break;
		}

		return Button::State::NORMAL;
	}

	bool UITeleportRock::select_at(Point<int16_t> cursor)
	{
		if (selected >= 0 && cursor.y() >= DESELECT_TOP && cursor.y() <= DESELECT_BOTTOM)
		{
			selected = -1;
			namefield.set_state(Textfield::State::FOCUSED);

			return true;
		}

		// v83 also hits the VIP rows scrolled out of view; only the shown ones are hit here.
		for (int16_t r = 0; r < ROWS; r++)
		{
			int16_t i = first + r;
			int16_t top = ROW_BAR.y() + ROW_PITCH * r;

			if (!Rectangle<int16_t>(ROW_BAR.x(), HIT_RIGHT, top, top + HIT_HEIGHT).contains(cursor))
				continue;

			if (i >= static_cast<int16_t>(maps.size()) || maps[i] == TeleportRock::EMPTY || i == selected)
				return false;

			// v83 gives the keys to Register.
			selected = i;
			release_field();

			return true;
		}

		return false;
	}

	void UITeleportRock::keep()
	{
		int32_t map_id = Stage::get().get_mapid();
		std::string problem = TeleportRock::keep_problem(maps, map_id);

		if (!problem.empty())
		{
			notice(problem);

			return;
		}

		ask("Will you enter this map\\nin your teleport list?\\n[" + map_name(map_id) + "]",
			[](UITeleportRock &window)
			{
				TeleportRockListPacket(window.vip).dispatch();
			}
		);
	}

	void UITeleportRock::drop()
	{
		if (selected < 0)
			return;

		int32_t map_id = maps[selected];

		ask("Will you delete this map from the\\nteleport list?\\n[" + map_name(map_id) + "]",
			[map_id](UITeleportRock &window)
			{
				TeleportRockListPacket(window.vip, map_id).dispatch();
				window.selected = -1;
			}
		);
	}

	void UITeleportRock::move()
	{
		// FUN_0083a3c2: a name is checked here and goes without asking.
		if (selected < 0)
		{
			std::string name = TeleportRock::trim(namefield.get_text());
			std::string problem = TeleportRock::target_problem(name, Stage::get().get_player().get_name());

			if (!problem.empty())
			{
				notice(problem);

				return;
			}

			close();
			onmove(name, TeleportRock::EMPTY);

			return;
		}

		int32_t map_id = maps[selected];

		if (map_id == Stage::get().get_mapid())
		{
			notice(TeleportRock::SAME_MAP);

			return;
		}

		ask("Will you teleport to this map?\\n[" + map_name(map_id) + "]",
			[map_id](UITeleportRock &window)
			{
				window.close();
				window.onmove("", map_id);
			}
		);
	}

	void UITeleportRock::close()
	{
		release_field();
		deactivate();
	}

	void UITeleportRock::notice(const std::string &text)
	{
		bool refocus = release_field();

		UI::get().emplace<UIOk>(
				text,
				[refocus](bool)
				{
					after_notice(refocus);
				}
		);
	}

	void UITeleportRock::ask(const std::string &question, std::function<void(UITeleportRock &)> on_yes)
	{
		bool refocus = release_field();

		UI::get().emplace<UIYesNo>(
				question,
				[refocus, on_yes](bool yes)
				{
					if (auto window = after_notice(refocus); window && yes)
						on_yes(*window);
				}
		);
	}

	Optional<UITeleportRock> UITeleportRock::after_notice(bool refocus)
	{
		auto window = UI::get().get_element<UITeleportRock>();

		if (!window || !window->is_active())
			return {};

		if (refocus)
			window->namefield.set_state(Textfield::State::FOCUSED);

		return window;
	}

	bool UITeleportRock::release_field()
	{
		if (namefield.get_state() != Textfield::State::FOCUSED)
			return false;

		UI::get().remove_textfield();

		return true;
	}
}
