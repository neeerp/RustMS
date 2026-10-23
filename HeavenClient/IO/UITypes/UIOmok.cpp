#include "UIOmok.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Net/Packets/MiniRoomPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The handicap (0x6e4773, b100 §1.2).
		constexpr Point<int16_t> BT_UNDO = Point<int16_t>(506, 403);

		// Where Omok's Draw puts the shared things (0x6e5215, b100 §1.3).
		const UIMiniGame::Layout LAYOUT = {
			Point<int16_t>(507, 22),
			{ 168, 186, 200, 214 },
			{ 450, 536 },
			{ 460, 546 },
			Point<int16_t>(490, 170),
			Point<int16_t>(13, 406),
			Point<int16_t>(300, 406)
		};

		// Each player's colour on their record panel (0x6e82a8).
		constexpr int16_t COLOR_X[2] = { 411, 497 };
		constexpr int16_t COLOR_Y = 207;

		// The board's clickable part (b100 §1.5).
		const Rectangle<int16_t> BOARD = Rectangle<int16_t>(11, 386, 19, 389);

		nl::node omok_node()
		{
			return NxFiles::ui_classic["UIWindow.img"]["Minigame"]["Omok"];
		}
	}

	UIOmok::UIOmok(const mini_room::Enter &enter) : UIMiniGame(enter, omok_node()["backgrnd"], LAYOUT),
			omok(enter.own_slot < 2 ? enter.own_slot : 0)
	{
		nl::node set = omok_node()["stone"][std::to_string(enter.piece)];
		stones[0] = set["black"];
		stones[1] = set["white"];

		nl::node Common = NxFiles::ui_classic["UIWindow.img"]["Minigame"]["Common"];
		buttons[Buttons::UNDO] = std::make_unique<MapleButton>(Common["btRefund"], BT_UNDO);

		refresh_buttons();
	}

	void UIOmok::start(uint8_t waits, std::vector<int32_t>)
	{
		omok.start(waits);
		refresh_buttons();
	}

	void UIOmok::turn(uint8_t slot)
	{
		omok.turn(slot);
	}

	void UIOmok::undo_asked()
	{
		UI::get().emplace<UIYesNo>("Your oppentent has requested to \\nwithdraw his/her last move.\\nDo you accept?", [](bool yes)
			{
				GamePacket(GamePacket::UNDO, yes).dispatch();
			});
	}

	void UIOmok::undo_result(bool undone, uint8_t count, uint8_t slot)
	{
		if (!undone)
		{
			omok.undo_refused();

			add_line("Your opponent denied your request.", Color::Name::BRIGHTRED);

			return;
		}

		omok.undo(count, slot);
	}

	void UIOmok::stone(const mini_game::Stone &stone)
	{
		omok.place(stone);

		play(stone.color == 2 ? "Slime" : "Mushroom");
	}

	void UIOmok::rejected(uint8_t code)
	{
		add_line(code == 'C' ? "You have double-3's." : "You can't put it there.", Color::Name::BRIGHTRED);
	}

	Button::State UIOmok::button_pressed(uint16_t buttonid)
	{
		if (buttonid != Buttons::UNDO)
			return UIMiniGame::button_pressed(buttonid);

		switch (omok.can_ask_undo())
		{
			case mini_game::Omok::Undo::USED:
				add_line("You can only request a handicap once per game.", Color::Name::BRIGHTRED);
				break;
			case mini_game::Omok::Undo::ASK:
				ask("Request to withdraw your last move?", [this]()
					{
						if (omok.can_ask_undo() == mini_game::Omok::Undo::ASK)
						{
							omok.ask_undo();
							GamePacket(GamePacket::ASK_UNDO).dispatch();
						}
					});

				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	mini_game::Game &UIOmok::game()
	{
		return omok;
	}

	const mini_game::Game &UIOmok::game() const
	{
		return omok;
	}

	void UIOmok::draw_board(float inter) const
	{
		for (const mini_game::Stone &stone : omok.stones)
		{
			auto at = mini_game::stone_position(stone.x, stone.y);

			stones[stone.color == 2].draw(position + Point<int16_t>(at.first, at.second), inter);
		}
	}

	void UIOmok::draw_counters(float inter) const
	{
		if (!omok.playing() && !omok.ended())
			return;

		// Each player's colour, mine on my side.
		uint8_t own_slot = get_own_slot();

		stones[omok.my_color == 2].draw(position + Point<int16_t>(COLOR_X[own_slot], COLOR_Y), inter);
		stones[omok.my_color != 2].draw(position + Point<int16_t>(COLOR_X[1 - own_slot], COLOR_Y), inter);
	}

	bool UIOmok::tick()
	{
		for (Animation &set : stones)
			set.update();

		return omok.tick(Constants::TIMESTEP);
	}

	int32_t UIOmok::seconds_left() const
	{
		return omok.seconds_left();
	}

	void UIOmok::click_board(Point<int16_t> at)
	{
		// A stone goes on an empty cell on my turn (0x6e4d1f).
		if (auto cell = mini_game::cell_at(at.x(), at.y()))
			if (omok.can_place(cell->first, cell->second))
				OmokMovePacket(cell->first, cell->second, omok.my_color).dispatch();
	}

	Rectangle<int16_t> UIOmok::board() const
	{
		return BOARD;
	}
}
