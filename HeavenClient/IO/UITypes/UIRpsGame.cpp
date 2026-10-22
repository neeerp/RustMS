#include "UIRpsGame.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/NpcInteractionPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Everything is from the window's top left (0x73d05b; every canvas's origin is 0, 0).
		const Point<int16_t> MAIN_AT(108, 202);
		const Point<int16_t> EXIT_AT(242, 335);
		const Point<int16_t> HAND_AT[3] = { { 19, 104 }, { 60, 104 }, { 101, 104 } };
		const Point<int16_t> OWN_HAND_AT(38, 20);
		const Point<int16_t> NPC_HAND_AT(190, 20);
		const Point<int16_t> BANNER_AT(31, 91);
		// Where the avatar's and the admin's feet go; the avatar faces the admin, who is drawn as he is.
		const Point<int16_t> AVATAR_AT(52, 261);
		const Point<int16_t> NPC_AT(256, 261);
		// The status line's 270 px (0x73fb0f): text that fits is centred, wider text starts halfway and scrolls
		// left a pixel a frame, coming back in from the right (0x73f630). v83 draws its cap top about 3 below,
		// we 9 for 12 px text.
		const Point<int16_t> STATUS_AT(20, 294);
		constexpr int16_t STATUS_WIDTH = 270;
		constexpr int16_t TEXT_RAISE = 6;
		// The top 15 px move the window (0x73e3f0).
		constexpr int16_t DRAG_HEIGHT = 15;

		// StringPool 0xe64 and 0xe63.
		constexpr const char* NO_MESOS =
			"You do not have enough mesos to pay for the participation fee of 1,000 mesos.";
		constexpr const char* NO_SLOT =
			"You may only participate in this game with at least one free slot available in the etc. inventory.";

		Expression::Id expression(int8_t emotion)
		{
			switch (emotion)
			{
				case 2:
					return Expression::Id::SMILE;
				case 3:
					return Expression::Id::TROUBLED;
				case 4:
					return Expression::Id::CRY;
				default:
					return Expression::Id::DEFAULT;
			}
		}
	}

	UIRpsGame::UIRpsGame(int32_t npc_id) : opened(ContinuousTimer::get().start()), emotion(Expression::Id::DEFAULT),
			status_x(0), dragged(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["RpsGame"];
		Texture backgrnd = src["backgrnd"];

		sprites.emplace_back(src["backgrnd"]);
		dimension = backgrnd.get_dimensions();

		buttons[Buttons::START] = std::make_unique<MapleButton>(src["BtStart"], MAIN_AT);
		buttons[Buttons::CONTINUE] = std::make_unique<MapleButton>(src["BtContinue"], MAIN_AT);
		buttons[Buttons::RETRY] = std::make_unique<MapleButton>(src["BtRetry"], MAIN_AT);
		buttons[Buttons::EXIT] = std::make_unique<MapleButton>(src["BtExit"], EXIT_AT);
		buttons[Buttons::ROCK] = std::make_unique<MapleButton>(src["BtRock"], HAND_AT[RpsGame::ROCK]);
		buttons[Buttons::PAPER] = std::make_unique<MapleButton>(src["BtPaper"], HAND_AT[RpsGame::PAPER]);
		buttons[Buttons::SCISSORS] = std::make_unique<MapleButton>(src["BtScissor"], HAND_AT[RpsGame::SCISSORS]);

		own_hands = { src["Frock"], src["Fpaper"], src["Fscissor"] };
		npc_hands = { src["rock"], src["paper"], src["scissor"] };
		banners = { Texture(), src["draw"], src["win"], src["lose"], src["timeover"] };
		npc = NxHelper::Npc::get_npc_node(npc_id)["stand"];

		status = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		// Centred on the screen.
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		show_buttons();
	}

	void UIRpsGame::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		own_hands[game.choice()].draw(position + OWN_HAND_AT);
		npc_hands[game.spin()].draw(position + NPC_HAND_AT);

		Stage::get().get_player().get_look().draw(position + AVATAR_AT, true, Stance::Id::STAND1, emotion);
		npc.draw(DrawArgument(position + NPC_AT, false), alpha);
		// Over the avatar and the admin (layer z 2 against their 1).
		banners[static_cast<size_t>(game.banner())].draw(position + BANNER_AT);

		Range<int16_t> line(position.x() + STATUS_AT.x(), position.x() + STATUS_AT.x() + STATUS_WIDTH);
		status.draw_between(position + STATUS_AT + Point<int16_t>(status_x, -TEXT_RAISE), line);

		UIElement::draw_buttons(alpha);
	}

	void UIRpsGame::update()
	{
		UIElement::update();
		npc.update();

		play(game.update(now()));
		show_buttons();

		if (game.update_status(now()))
		{
			status.change_text(game.status());
			int16_t width = status.width();
			status_x = width > STATUS_WIDTH ? STATUS_WIDTH / 2 : (STATUS_WIDTH - width) / 2;
		}
		else if (status.width() > STATUS_WIDTH && --status_x + status.width() < 0)
		{
			status_x = STATUS_WIDTH;
		}
	}

	Cursor::State UIRpsGame::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (clicked && dragged)
		{
			position = cursorpos - drag_from;

			return Cursor::State::CLICKING;
		}

		dragged = false;
		Cursor::State state = UIElement::send_cursor(clicked, cursorpos);

		if (clicked && !is_holding_button() && is_in_range(cursorpos) && cursorpos.y() - position.y() < DRAG_HEIGHT)
		{
			drag_from = cursorpos - position;
			dragged = true;
		}

		return state;
	}

	void UIRpsGame::remove_cursor()
	{
		UIElement::remove_cursor();
		dragged = false;
	}

	UIElement::Type UIRpsGame::get_type() const
	{
		return TYPE;
	}

	void UIRpsGame::refused(Refusal refusal)
	{
		game.reset();
		show_buttons();

		if (refusal == Refusal::NO_MESOS)
			UI::get().emplace<UIOk>(NO_MESOS, [](bool) {});
		else if (refusal == Refusal::NO_SLOT)
			UI::get().emplace<UIOk>(NO_SLOT, [](bool) {});
	}

	void UIRpsGame::start_round()
	{
		game.start_round(now());
		show_buttons();
	}

	void UIRpsGame::time_over()
	{
		game.time_over(now());
		show_buttons();
	}

	void UIRpsGame::result(int8_t hand, int8_t streak)
	{
		game.result(hand, streak);
	}

	void UIRpsGame::close()
	{
		deactivate();
	}

	Button::State UIRpsGame::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::START:
			case Buttons::CONTINUE:
			case Buttons::RETRY:
			case Buttons::EXIT:
			{
				static constexpr RpsActionPacket::Mode MODES[] = {
					RpsActionPacket::Mode::START, RpsActionPacket::Mode::CONTINUE, RpsActionPacket::Mode::RETRY,
					RpsActionPacket::Mode::EXIT
				};

				game.pressed();
				RpsActionPacket(MODES[buttonid]).dispatch();
				break;
			}
			default:
			{
				int8_t hand = static_cast<int8_t>(buttonid - Buttons::ROCK);
				game.chose(hand);
				RpsActionPacket(hand).dispatch();
				break;
			}
		}

		show_buttons();

		return Button::State::NORMAL;
	}

	// Ticks in ms since the window opened, never 0.
	int64_t UIRpsGame::now() const
	{
		return ContinuousTimer::get().stop(opened) / 1000 + 1;
	}

	void UIRpsGame::show_buttons()
	{
		RpsGame::Main main = game.main();
		bool main_shown = game.main_shown();

		buttons[Buttons::START]->set_active(main_shown && main == RpsGame::Main::START);
		buttons[Buttons::CONTINUE]->set_active(main_shown && main == RpsGame::Main::CONTINUE);
		buttons[Buttons::RETRY]->set_active(main_shown && main == RpsGame::Main::RETRY);
		buttons[Buttons::EXIT]->set_active(game.exit_shown());

		for (uint16_t hand = Buttons::ROCK; hand <= Buttons::SCISSORS; hand++)
			buttons[hand]->set_active(game.hands_shown());
	}

	void UIRpsGame::play(RpsGame::Events events)
	{
		nl::node minigame = nl::nx::sound["MiniGame.img"];

		if (events.reel)
			Sound(minigame["Reel"]).play();

		if (events.timer)
			Sound(minigame["Timer"]).play();

		switch (events.jingle)
		{
			case RpsGame::Jingle::WIN:
				Sound(minigame["Win"]).play();
				break;
			case RpsGame::Jingle::DRAW:
				Sound(minigame["Draw"]).play();
				break;
			case RpsGame::Jingle::LOOSE:
				Sound(minigame["Loose"]).play();
				break;
			case RpsGame::Jingle::NONE:
				break;
		}

		if (events.emotion >= 0)
			emotion = expression(events.emotion);

		if (events.time_up)
			RpsActionPacket(RpsActionPacket::Mode::TIME_UP).dispatch();
	}
}
