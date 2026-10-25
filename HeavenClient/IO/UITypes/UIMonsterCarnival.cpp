#include "UIMonsterCarnival.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <GLFW/glfw3.h>
#include <algorithm>
#include <cstring>
#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		using namespace carnival::window;

		// v83 draws a line's cap top about 3 below where it is drawn, we 9.
		constexpr Point<int16_t> TEXT_Y = Point<int16_t>(0, -6);

		constexpr Point<int16_t> TOP = Point<int16_t>(0, 104);
		constexpr Point<int16_t> MY_CP = Point<int16_t>(79, 35);
		constexpr Point<int16_t> OUR_ICON = Point<int16_t>(8, 51);
		constexpr Point<int16_t> OUR_CP = Point<int16_t>(79, 55);
		constexpr Point<int16_t> THEIR_ICON = Point<int16_t>(8, 73);
		constexpr Point<int16_t> THEIR_CP = Point<int16_t>(79, 75);

		// The tab strip (style 5): each image centred on its tab's span,
		// and in the 21 px strip less 1.
		constexpr Point<int16_t> STRIP = Point<int16_t>(4, 107);
		constexpr int16_t STRIP_HEIGHT = 21;
		constexpr int16_t TAB_STARTS[TABS] = { 3, 28, 65 };
		constexpr int16_t TAB_ENDS[TABS] = { 20, 57, 106 };

		// A row's texts, from its top, and the selected row's fill.
		constexpr int16_t NAME_X = 10;
		constexpr int16_t NAME_WIDTH = 70;
		constexpr int16_t KEY_X = 116;
		constexpr Point<int16_t> FILL = Point<int16_t>(5, 0);
		constexpr Point<int16_t> FILL_SIZE = Point<int16_t>(133, ROW_HEIGHT);
		constexpr Color SELECTED = Color(0x244768FFu);

		constexpr int16_t SLIDER_X = 142;
		constexpr int16_t LOCK_X = 9;
		constexpr int16_t DECIDE_X = 110;

		// Tooltips go 20 right of and below the cursor (A 870423).
		constexpr Point<int16_t> TIP_OFFSET = Point<int16_t>(20, 20);
		// The CP lines' rects, whose tooltips explain them (FUN_008700b4).
		constexpr int16_t CP_LEFT = 9, CP_RIGHT = 139;
		constexpr int16_t CP_TOPS[3] = { 35, 55, 75 };
		constexpr int16_t CP_LINE = 10;

		// StringPool 0x1020-0x1024 and 0x1050/0x1051.
		const char* CP_TIPS[3] = {
			"CP earned that can be used.",
			"The game will be decided based on the total CP earned between two teams.",
			"Compare the total CP earned between two teams to find the winner."
		};
		const char* LOCK_ON = "Automatic lock enabled.";
		const char* LOCK_OFF = "Automatic lock disabled.";

		// v83 copies each canvas with its top left at the given point.
		void draw_at(const Texture& texture, Point<int16_t> corner)
		{
			texture.draw(DrawArgument(corner + texture.get_origin()));
		}

		std::string pair(carnival::Points points)
		{
			return std::to_string(points.cp) + " / " + std::to_string(points.total);
		}

		// SP 0x1021.
		std::string remaining(carnival::Points points)
		{
			return "CP Remaining: " + std::to_string(points.cp) + " / CP Earned: " + std::to_string(points.total);
		}

		std::string trimmed(std::string text)
		{
			size_t first = text.find_first_not_of(' ');

			return first == std::string::npos ? "" : text.substr(first);
		}

		Text black(const std::string& text)
		{
			return Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK, text, 0, false);
		}

		Text white(const std::string& text)
		{
			return Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, text, 0, false);
		}
	}

	UIMonsterCarnival::UIMonsterCarnival()
			: UIDragElement<PosMONSTERCARNIVAL>(Point<int16_t>(WIDTH, 20)), shown_team(carnival::NO_TEAM),
			  hovered(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["MonsterCarnival"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		background = src["backgrnd"];
		top = src["backgrnd3"]["top"]["0"];
		header = src["backgrnd3"]["middle0"]["0"];
		middle = src["backgrnd3"]["middle1"]["0"];
		bottom = src["backgrnd3"]["bottom"]["0"];
		team_icons = { src["icon0"], src["icon1"] };
		lock_icons = { src["icon3"], src["icon2"] };

		for (uint8_t tab = 0; tab < TABS; tab++)
		{
			std::string index = std::to_string(tab);
			tab_on[tab] = src["Tab"]["enabled"][index];
			tab_off[tab] = src["Tab"]["disabled"][index];
		}

		buttons[BT_MAX] = std::make_unique<MapleButton>(basic["BtMax"], Point<int16_t>(140, 8));
		buttons[BT_MIN] = std::make_unique<MapleButton>(basic["BtMin"], Point<int16_t>(126, 8));
		buttons[BT_SIDE] = std::make_unique<MapleButton>(src["BtSide"], Point<int16_t>(149, 28));
		buttons[BT_DECIDE] = std::make_unique<MapleButton>(basic["BtDecide"], Point<int16_t>(DECIDE_X, 0));

		// The rows come from the field's monsterCarnival lists, as START
		// has v83 fill them (FUN_00565156).
		nl::node lists = NxHelper::Map::get_map_node_name(carnival::field_map())["monsterCarnival"];

		for (nl::node entry : lists["mob"])
		{
			std::string id = std::to_string(static_cast<int32_t>(entry["id"]));
			add_row(0, nl::nx::string["Mob.img"][id]["name"], entry["spendCP"], "");
		}

		for (nl::node entry : lists["skill"])
		{
			nl::node skill = nl::nx::skill["MCSkill.img"][std::to_string(static_cast<int32_t>(entry))];
			add_row(1, skill["name"], skill["spendCP"], skill["desc"]);
		}

		for (nl::node entry : lists["guardian"])
		{
			nl::node guardian = nl::nx::skill["MCGuardian.img"][std::to_string(static_cast<int32_t>(entry))];
			add_row(2, guardian["name"], guardian["spendCP"], guardian["desc"]);
		}

		state = State({ rows[0].size(), rows[1].size(), rows[2].size() });

		my_cp = black("");
		our_cp = black("");
		their_cp = black("");
		refresh_points();
		relayout();
	}

	void UIMonsterCarnival::show()
	{
		if (!UI::get().get_element<UIMonsterCarnival>())
			UI::get().emplace<UIMonsterCarnival>();
	}

	void UIMonsterCarnival::add_row(uint8_t tab, const std::string& name, int32_t cost, const std::string& desc)
	{
		std::vector<Row>& list = rows[tab];
		std::string shown_name = trimmed(name);
		std::string shown_cost = std::to_string(cost);
		// Each row's number is its F-key (FUN_0086c9ec).
		std::string key = "F" + std::to_string(list.size() + 1);

		Row row;
		row.name = { black(shown_name), white(shown_name) };
		row.cost = { black(shown_cost), white(shown_cost) };
		row.key = { black(key), white(key) };
		row.cost_x = cost_x(cost);
		row.desc = desc;

		list.push_back(std::move(row));
	}

	void UIMonsterCarnival::relayout()
	{
		size_t shown = state.shown();
		int16_t list_height = static_cast<int16_t>(ROW_HEIGHT * shown + 19);

		dimension = Point<int16_t>(WIDTH, state.height());

		buttons[BT_MAX]->set_active(!state.open());
		buttons[BT_MIN]->set_active(state.open());
		buttons[BT_DECIDE]->set_active(state.deciding());
		buttons[BT_DECIDE]->set_position(Point<int16_t>(DECIDE_X, static_cast<int16_t>(ROW_HEIGHT * shown + 159)));

		// v83's list bar (skin 3, VScr4) never has anything to scroll: no tab
		// has more rows than it shows.
		int16_t arrow = 13;
		slider = Slider(Slider::Type::CLASSIC, Range<int16_t>(LIST_Y, LIST_Y + list_height - arrow), SLIDER_X,
						static_cast<int16_t>(MAX_ROWS), static_cast<int16_t>(shown), [](bool) {});
	}

	void UIMonsterCarnival::refresh_points()
	{
		const carnival::State& now = carnival::state();
		auto same = [](carnival::Points a, carnival::Points b) { return a.cp == b.cp && a.total == b.total; };

		if (!my_cp.empty() && now.team == shown_team && same(now.mine, shown_mine) &&
			same(now.sides[0], shown_sides[0]) && same(now.sides[1], shown_sides[1]))
			return;

		int8_t team = now.team == 0 ? 0 : 1;
		my_cp.change_text(pair(now.mine));
		our_cp.change_text(pair(now.sides[team]));
		their_cp.change_text(pair(now.sides[1 - team]));

		shown_mine = now.mine;
		shown_sides = now.sides;
		shown_team = now.team;
	}

	void UIMonsterCarnival::draw(float alpha) const
	{
		const carnival::State& now = carnival::state();
		// Red's icon is icon0 and Blue's icon1 (A 86f667).
		int8_t team = now.team == 0 ? 0 : 1;

		if (state.open())
			draw_at(top, position + TOP);

		draw_at(background, position);

		my_cp.draw(position + MY_CP + TEXT_Y);
		draw_at(team_icons[team], position + OUR_ICON);
		our_cp.draw(position + OUR_CP + TEXT_Y);
		draw_at(team_icons[1 - team], position + THEIR_ICON);
		their_cp.draw(position + THEIR_CP + TEXT_Y);

		for (uint8_t tab = 0; tab < TABS; tab++)
		{
			const Texture& image = tab == state.tab() ? tab_on[tab] : tab_off[tab];
			Point<int16_t> size = image.get_dimensions();
			int16_t x = STRIP.x() + TAB_STARTS[tab] + ((TAB_ENDS[tab] - TAB_STARTS[tab]) / 2 - size.x() / 2);
			int16_t y = STRIP.y() + (STRIP_HEIGHT / 2 - size.y() / 2) - 1;

			draw_at(image, position + Point<int16_t>(x, y));
		}

		if (state.open())
		{
			size_t shown = state.shown();
			const std::vector<Row>& list = rows[state.tab()];

			draw_at(header, position + Point<int16_t>(0, LIST_Y));

			for (size_t i = 0; i < shown; i++)
			{
				const Row& row = list[i];
				Point<int16_t> corner = position + Point<int16_t>(0, static_cast<int16_t>(ROWS_Y + ROW_HEIGHT * i));
				bool selected = state.selected() == i + 1;

				draw_at(middle, corner);

				if (selected)
				{
					Point<int16_t> fill = corner + FILL;
					GraphicsGL::get().drawrectangle(fill.x(), fill.y(), FILL_SIZE.x(), FILL_SIZE.y(), SELECTED.r(),
													SELECTED.g(), SELECTED.b(), SELECTED.a());
				}

				// The texts start a pixel under the row's top.
				Point<int16_t> line = corner + Point<int16_t>(0, 1) + TEXT_Y;
				int16_t name_x = line.x() + NAME_X;

				row.name[selected].draw_between(DrawArgument(Point<int16_t>(name_x, line.y())),
												Range<int16_t>(name_x, name_x + NAME_WIDTH));
				row.cost[selected].draw(line + Point<int16_t>(row.cost_x, 0));
				row.key[selected].draw(line + Point<int16_t>(KEY_X, 0));
			}

			draw_at(bottom, position + Point<int16_t>(0, static_cast<int16_t>(ROWS_Y + ROW_HEIGHT * shown)));
			slider.draw(position);
			draw_at(lock_icons[state.locked()],
					position + Point<int16_t>(LOCK_X, static_cast<int16_t>(ROW_HEIGHT * shown + 162)));
		}

		UIElement::draw(alpha);

		if (hovered)
			tip.draw(cursor + TIP_OFFSET);
	}

	void UIMonsterCarnival::update()
	{
		UIElement::update();

		if (state.pass(Constants::TIMESTEP))
		{
			tip.set_text("");
			relayout();
		}

		refresh_points();
	}

	bool UIMonsterCarnival::on_button(Point<int16_t> cursorpos) const
	{
		for (auto& button : buttons)
			if (button.second->is_active() && button.second->bounds(position).contains(cursorpos))
				return true;

		return false;
	}

	void UIMonsterCarnival::show_tip(Point<int16_t> relative)
	{
		const carnival::State& now = carnival::state();
		int8_t team = now.team == 0 ? 0 : 1;
		carnival::Points lines[3] = { now.mine, now.sides[team], now.sides[1 - team] };

		for (size_t i = 0; i < 3; i++)
		{
			if (relative.x() >= CP_LEFT && relative.x() < CP_RIGHT && relative.y() >= CP_TOPS[i] &&
				relative.y() < CP_TOPS[i] + CP_LINE)
			{
				// v83 makes the box 6 px a character of its longer line (A 8702ec).
				std::string second = remaining(lines[i]);
				size_t longest = std::max(std::strlen(CP_TIPS[i]), second.size());
				tip.set_text(std::string(CP_TIPS[i]) + "\\n" + second, static_cast<int16_t>(6 * longest));
				return;
			}
		}

		if (state.open() && on_lock(relative, state.shown()))
		{
			tip.set_text(state.locked() ? LOCK_ON : LOCK_OFF);
			return;
		}

		// A row's description, over any row, keeps the window open
		// (FUN_008703e1).
		if (state.open())
		{
			if (auto row = row_at(relative, state.shown()))
			{
				const std::string& desc = rows[state.tab()][*row].desc;

				if (!desc.empty())
				{
					tip.set_text(desc);
					state.touch();
					return;
				}
			}
		}

		tip.set_text("");
	}

	Cursor::State UIMonsterCarnival::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (dragged)
			return UIDragElement::send_cursor(clicked, cursorpos);

		Point<int16_t> relative = cursorpos - position;
		// Only a press acts, not the button held while the mouse moves.
		bool pressed = clicked && UI::get().is_cursor_press();

		hovered = true;
		cursor = cursorpos;
		show_tip(relative);

		if (state.open())
			if (Cursor::State slid = slider.send_cursor(relative, clicked))
				return slid;

		if (on_button(cursorpos))
			return UIDragElement::send_cursor(clicked, cursorpos);

		if (auto tab = tab_at(relative))
		{
			if (!pressed)
				return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;

			Sound(Sound::Name::TAB).play();
			select_tab(*tab);

			return Cursor::State::CLICKING;
		}

		if (pressed && state.open())
		{
			if (on_lock(relative, state.shown()))
			{
				state.toggle_lock();
				show_tip(relative);

				return Cursor::State::CLICKING;
			}

			// A click anywhere else picks the row under it, or none
			// (FUN_0086fe6f).
			if (state.click(row_at(relative, state.shown())) || buttons[BT_DECIDE]->is_active() != state.deciding())
				relayout();
		}

		return UIDragElement::send_cursor(clicked, cursorpos);
	}

	void UIMonsterCarnival::remove_cursor()
	{
		hovered = false;
		tip.set_text("");
		slider.remove_cursor();

		UIDragElement::remove_cursor();
	}

	void UIMonsterCarnival::doubleclick(Point<int16_t> cursorpos)
	{
		// The first click of the two picked the row; the second spends. The
		// scrollbar over the rows' right end takes its own clicks.
		Point<int16_t> relative = cursorpos - position;

		if (!state.open() || on_button(cursorpos) || relative.x() >= SLIDER_X)
			return;

		if (auto row = row_at(relative, state.shown()))
		{
			state.click(row);
			relayout();
			spend();
		}
	}

	bool UIMonsterCarnival::send_hotkey(int32_t keycode, bool pressed)
	{
		bool tab = keycode == GLFW_KEY_TAB;
		bool fkey = keycode >= GLFW_KEY_F1 && keycode <= GLFW_KEY_F12;

		if (!tab && !fkey)
			return false;

		if (!pressed)
			return true;

		if (tab)
		{
			Sound(Sound::Name::TAB).play();
			state.tab_key();
			relayout();
		}
		else if (state.hotkey(static_cast<size_t>(keycode - GLFW_KEY_F1)))
		{
			Sound(Sound::Name::TAB).play();
			relayout();
			spend();
		}

		return true;
	}

	void UIMonsterCarnival::select_tab(uint8_t tab)
	{
		state.choose_tab(tab);
		relayout();
	}

	void UIMonsterCarnival::spend() const
	{
		if (state.selected() == 0)
			return;

		uint8_t index = static_cast<uint8_t>(state.selected() - 1);

		// One request at a time, from the living, 500 ms apart (FUN_00485bf7).
		Stage::get().get_player().request(MonsterCarnivalPacket(state.tab(), index), 500);
	}

	Button::State UIMonsterCarnival::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case BT_MAX:
			state.open_up();
			relayout();
			break;
		case BT_MIN:
			state.fold();
			tip.set_text("");
			relayout();
			break;
		case BT_DECIDE:
			state.touch();
			spend();
			break;
		default:
			// BtSide's summon list isn't in yet.
			break;
		}

		return Button::State::NORMAL;
	}

	UIElement::Type UIMonsterCarnival::get_type() const
	{
		return TYPE;
	}
}
