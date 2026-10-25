#include "Carnival.h"

#include <algorithm>

namespace ms
{
	namespace carnival
	{
		namespace
		{
			int32_t entered = 0;
			int32_t entered_map = 0;

			bool has_side(int8_t team)
			{
				return team == 0 || team == 1;
			}
		}

		bool reads_team(int32_t fieldtype)
		{
			return fieldtype == 10 || fieldtype == 11 || fieldtype == 15;
		}

		bool battle_field(int32_t fieldtype)
		{
			return fieldtype == 10 || fieldtype == 15;
		}

		void enter_field(int32_t mapid, int32_t fieldtype)
		{
			entered_map = mapid;
			entered = fieldtype;
			reset();
		}

		int32_t field_type()
		{
			return entered;
		}

		int32_t field_map()
		{
			return entered_map;
		}

		std::string team_name(int8_t team)
		{
			switch (team)
			{
			case 0:
				return "Maple Red";
			case 1:
				return "Maple Blue";
			default:
				return "";
			}
		}

		Points read_points(InPacket& recv)
		{
			Points points;
			points.cp = static_cast<uint16_t>(recv.read_short());
			points.total = static_cast<uint16_t>(recv.read_short());

			return points;
		}

		Start read_start(InPacket& recv)
		{
			Start start;
			start.team = recv.read_byte();
			start.mine = read_points(recv);
			start.ours = read_points(recv);
			start.theirs = read_points(recv);

			// One for each of the field's mob list; the server sends as many.
			while (recv.available())
				start.summoned.push_back(static_cast<uint8_t>(recv.read_byte()));

			return start;
		}

		State& state()
		{
			static State carnival;

			return carnival;
		}

		void reset()
		{
			state() = {};
		}

		void start(const Start& start)
		{
			State& now = state();
			now.team = start.team;
			now.battle = true;
			now.mine = start.mine;
			now.summoned = start.summoned;

			if (has_side(start.team))
			{
				now.sides[start.team] = start.ours;
				now.sides[1 - start.team] = start.theirs;
			}
		}

		bool ally(int8_t mob_team)
		{
			const State& now = state();

			return now.battle && has_side(now.team) && mob_team == now.team;
		}

		std::string summon_line(uint8_t tab, const std::string& who, const std::string& what)
		{
			switch (tab)
			{
			case 0:
				return "[" + who + "] has summoned a being. [" + what + "]";
			case 1:
				return "[" + who + "] has used a skill. [" + what + "]";
			default:
				return "[" + who + "] has summoned the being. [" + what + "]";
			}
		}

		std::optional<std::string> message_line(uint8_t code)
		{
			switch (code)
			{
			case 1:
				return "You don't have enough CP to continue.";
			case 2:
				return "You can no longer summon the Monster.";
			case 3:
				return "You can no longer summon the being.";
			case 4:
				return "This being is already summoned.";
			case 5:
				return "This request has failed due to an unknown error.";
			default:
				return std::nullopt;
			}
		}

		std::string died_line(int8_t team, const std::string& name, uint8_t cp)
		{
			// Anything but Red is Blue here.
			std::string side = team == 0 ? team_name(0) : team_name(1);

			if (cp != 0)
				return "[" + name + "] has become unable to fight and [" + side + "]team has lost " +
					std::to_string(cp) + " CP.";

			return "[" + name + "] has become unable to fight but [" + side + "] has no CP so [" + side +
				"] team did not lose any CP";
		}

		std::string leave_line(uint8_t type, int8_t team, const std::string& name)
		{
			std::string side = team == 0 ? team_name(0) : team_name(1);

			// v83 passes the side first and the name second to both (A
			// 565a42, 565a79), so the first reads "[Maple Red] of Team [name]".
			if (type == 6)
				return "Since the leader of the Team [" + side + "] quit the Monster Carnival, [" + name +
					"] has been appointed as the new leader of the team.";

			return "[" + side + "] of Team [" + name + "] has quit the Monster Carnival.";
		}

		std::optional<std::string> result_line(uint8_t code)
		{
			const std::string wait = " Please wait as you'll be transported out of here shortly.";

			switch (code)
			{
			case 8:
				return "You have won the Monster Carnival." + wait;
			case 9:
				return "Unfortunately, you have lost the Monster Carnival." + wait;
			case 10:
				return "Despite the Overtime, the carnival ended in a draw." + wait;
			case 11:
				return "Monster Carnival has ended abruptly due to the opposing team leaving the game too early." +
					wait;
			default:
				return std::nullopt;
			}
		}

		namespace window
		{
			size_t shown(size_t rows)
			{
				return rows < MAX_ROWS ? rows : MAX_ROWS;
			}

			int16_t height(bool open, size_t rows)
			{
				if (!open)
					return FOLDED_HEIGHT;

				return static_cast<int16_t>(ROW_HEIGHT * (14 + shown(rows)));
			}

			int16_t cost_x(int32_t cost)
			{
				return static_cast<int16_t>(101 - 5 * std::to_string(cost).size());
			}

			std::optional<size_t> row_at(Point<int16_t> pos, size_t shown)
			{
				// The rows' rects are x 0..138 and y 19 + 13i .. 32 + 13i of
				// the list, tested at (x - 6, y - 128) (A 86fe80).
				if (pos.x() < 6 || pos.x() >= 6 + 138 || pos.y() < ROWS_Y)
					return std::nullopt;

				size_t row = static_cast<size_t>((pos.y() - ROWS_Y) / ROW_HEIGHT);

				if (row >= shown)
					return std::nullopt;

				return row;
			}

			std::optional<uint8_t> tab_at(Point<int16_t> pos)
			{
				// The strip is at (4, 107), 21 high; the tabs span 3..20,
				// 28..57 and 65..106 of it, and take a click 2 px either side
				// from 4 px down (style 5).
				constexpr int16_t STRIP_X = 4, STRIP_Y = 107, STRIP_HEIGHT = 21;
				constexpr int16_t STARTS[TABS] = { 3, 28, 65 };
				constexpr int16_t ENDS[TABS] = { 20, 57, 106 };

				int16_t x = pos.x() - STRIP_X;
				int16_t y = pos.y() - STRIP_Y;

				if (y < 4 || y >= STRIP_HEIGHT)
					return std::nullopt;

				for (uint8_t tab = 0; tab < TABS; tab++)
					if (x >= STARTS[tab] - 2 && x <= ENDS[tab] + 2)
						return tab;

				return std::nullopt;
			}

			bool on_lock(Point<int16_t> pos, size_t shown)
			{
				int16_t top = static_cast<int16_t>(ROW_HEIGHT * shown + 162);

				return pos.x() >= 9 && pos.x() < 18 && pos.y() >= top && pos.y() < top + 12;
			}

			State::State(const std::array<size_t, TABS>& r) : rows(r)
			{}

			uint8_t State::tab() const
			{
				return current;
			}

			size_t State::selected() const
			{
				return chosen;
			}

			bool State::open() const
			{
				return is_open;
			}

			bool State::locked() const
			{
				return is_locked;
			}

			size_t State::shown() const
			{
				return window::shown(rows[current]);
			}

			int16_t State::height() const
			{
				return window::height(is_open, rows[current]);
			}

			bool State::deciding() const
			{
				return is_open && shown() > 0 && chosen != 0;
			}

			void State::choose_tab(uint8_t tab)
			{
				if (tab >= TABS)
					return;

				if (tab != current)
					chosen = 0;

				current = tab;
				open_up();
			}

			void State::tab_key()
			{
				choose_tab(is_open ? static_cast<uint8_t>((current + 1) % TABS) : current);
			}

			std::optional<size_t> State::hotkey(size_t key)
			{
				if (key >= MAX_ROWS || key >= rows[current])
					return std::nullopt;

				chosen = key + 1;
				choose_tab(current);

				return key;
			}

			bool State::click(std::optional<size_t> row)
			{
				size_t before = chosen;
				chosen = row ? *row + 1 : 0;

				if (row)
					touch();

				return chosen != before;
			}

			void State::toggle_lock()
			{
				is_locked = !is_locked;
				touch();
			}

			void State::open_up()
			{
				is_open = true;
				touch();
			}

			void State::fold()
			{
				is_open = false;
			}

			void State::touch()
			{
				timer = FOLD_DELAY;
			}

			bool State::pass(int32_t ms)
			{
				if (is_locked || !is_open)
					return false;

				timer -= ms;

				if (timer >= 0)
					return false;

				fold();

				return true;
			}

			void Tally::add(const std::string& name, uint32_t count)
			{
				rows.push_back({ name, count });
			}

			size_t Tally::summon(const std::string& name)
			{
				last = name;

				for (size_t i = 0; i < rows.size(); i++)
				{
					if (rows[i].name == name)
					{
						rows[i].count++;

						return i;
					}
				}

				rows.push_back({ name, 1 });

				return rows.size() - 1;
			}

			const std::vector<Tally::Entry>& Tally::entries() const
			{
				return rows;
			}

			const std::string& Tally::latest() const
			{
				return last;
			}

			size_t Tally::top() const
			{
				return first;
			}

			size_t Tally::range() const
			{
				// v83's count less 4, not 5: a row past the last full page.
				return rows.size() > SIDE_ROWS ? rows.size() - (SIDE_ROWS - 1) : 0;
			}

			void Tally::scroll(size_t top)
			{
				first = std::min(top, range());
			}
		}
	}
}
