#include "UIClaim.h"

#include "UIChatBar.h"
#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"
#include "../Components/MapleComboBox.h"

#include "../../Constants.h"
#include "../../Data/CurseData.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/PlayerInteractionPackets.h"
#include "../../Util/NxFiles.h"

#include <ctime>
#include <functional>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The controls (b155-s2 §1, §2).
		constexpr Point<int16_t> CONVERSATION_AT = Point<int16_t>(140, 197);
		constexpr Point<int16_t> PROGRAM_AT = Point<int16_t>(140, 269);
		constexpr Point<int16_t> NOTICE_CANCEL_AT = Point<int16_t>(200, 402);
		constexpr Point<int16_t> CHARACTER_AT = Point<int16_t>(107, 158);
		constexpr Point<int16_t> REASON_AT = Point<int16_t>(107, 178);
		constexpr Point<int16_t> OK_AT = Point<int16_t>(40, 320);
		constexpr Point<int16_t> CANCEL_AT = Point<int16_t>(180, 320);
		// The combos are 155 wide, their caps 5 and 18.
		constexpr int64_t COMBO_MIDDLE = 132;
		// The description's lines, 145 by 15 from (110, 200).
		constexpr int16_t LINE_LEFT = 110;
		constexpr int16_t LINE_WIDTH = 145;
		constexpr int16_t LINE_TOP = 200;
		constexpr int16_t LINE_HEIGHT = 15;

		// Centred, as v83's dialogs are (0x4edaef).
		Point<int16_t> centred(Point<int16_t> dimension)
		{
			return Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
					(Constants::Constants::get().get_viewheight() - dimension.y()) / 2);
		}

		void tell(const std::string &message, std::function<void()> then = [] {})
		{
			UI::get().emplace<UIOk>(message, [then](bool) { then(); });
		}
	}

	UIClaimNotice::UIClaimNotice(const std::string &n, const std::vector<std::string> &r) :
			UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), name(n), rows(r)
	{
		nl::node Claim = NxFiles::ui_classic["UIWindow.img"]["Claim"];

		backgrnd = Claim["notice"];
		dimension = backgrnd.get_dimensions();
		position = centred(dimension);

		buttons[Buttons::CONVERSATION] = std::make_unique<MapleButton>(Claim["BtCClaim"], CONVERSATION_AT);
		buttons[Buttons::PROGRAM] = std::make_unique<MapleButton>(Claim["BtPClaim"], PROGRAM_AT);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(Claim["BtCancel"], NOTICE_CANCEL_AT);
	}

	void UIClaimNotice::draw(float inter) const
	{
		backgrnd.draw(position);

		UIElement::draw(inter);
	}

	void UIClaimNotice::send_key(int32_t, bool pressed, bool escape)
	{
		// Only Esc does anything; there is no OK (0x4edd21).
		if (pressed && escape)
			deactivate();
	}

	UIElement::Type UIClaimNotice::get_type() const
	{
		return TYPE;
	}

	void UIClaimNotice::open(const std::string &name)
	{
		std::time_t now = std::time(nullptr);

		if (auto refusal = Claim::refusal(Claim::open, Claim::start_hour, Claim::end_hour,
				std::localtime(&now)->tm_hour))
		{
			tell(*refusal);

			return;
		}

		std::vector<std::string> rows;

		if (auto chatbar = UI::get().get_element<UIChatBar>())
			rows = chatbar->claim_rows();

		UI::get().emplace<UIClaimNotice>(name, rows);
	}

	Button::State UIClaimNotice::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::CONVERSATION:
				choose(Claim::Kind::CONVERSATION);
				break;
			case Buttons::PROGRAM:
				choose(Claim::Kind::PROGRAM);
				break;
			default:
				deactivate();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIClaimNotice::choose(Claim::Kind kind)
	{
		deactivate();

		const std::string self = Stage::get().get_player().get_stats().get_name();
		std::vector<std::string> names;

		if (kind == Claim::Kind::CONVERSATION)
		{
			// Only someone who spoke in the log (0x490927, 0x490a28).
			std::vector<std::string> speakers = Claim::speakers(rows, self);

			if (!name.empty())
			{
				if (std::find(speakers.begin(), speakers.end(), name) == speakers.end())
				{
					tell("This character is unable to report others.");

					return;
				}

				names = { name };
			}
			else
			{
				names = speakers;
			}
		}
		else
		{
			// Only someone on the map, by name (0xa2719c).
			for (auto &mmo : *Stage::get().get_chars().get_chars())
				names.push_back(static_cast<const Char *>(mmo.second.get())->get_name());

			if (!name.empty())
			{
				if (std::find(names.begin(), names.end(), name) == names.end())
					return;

				names = { name };
			}
			else
			{
				names = Claim::sorted(names);
			}
		}

		if (names.empty())
		{
			tell("There's no character can be reported in this map.");

			return;
		}

		UI::get().emplace<UIClaim>(kind, names, rows);
	}

	UIClaim::UIClaim(Claim::Kind k, const std::vector<std::string> &n, const std::vector<std::string> &r) :
			UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), kind(k), names(n), rows(r)
	{
		nl::node Claim = NxFiles::ui_classic["UIWindow.img"]["Claim"];

		backgrnd = Claim["backgrnd"];
		dimension = backgrnd.get_dimensions();
		position = centred(dimension);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(Claim["BtClaim"], OK_AT);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(Claim["BtCancel"], CANCEL_AT);

		// "Please select." comes first, and stays chosen until another is
		// (0x7daf89).
		std::vector<std::string> characters = { "Please select." };
		characters.insert(characters.end(), names.begin(), names.end());

		buttons[Buttons::CHARACTER] = std::make_unique<MapleComboBox>(MapleComboBox::Type::DEFAULT, characters, 0,
				position, CHARACTER_AT, COMBO_MIDDLE, true);
		buttons[Buttons::REASON] = std::make_unique<MapleComboBox>(MapleComboBox::Type::DEFAULT, Claim::reasons(kind),
				0, position, REASON_AT, COMBO_MIDDLE, true);

		// Enter and the arrows move between the lines (0x7db747).
		for (size_t i = 0; i < lines.size(); i++)
		{
			int16_t top = LINE_TOP + static_cast<int16_t>(i) * LINE_HEIGHT;

			lines[i] = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
					Rectangle<int16_t>(LINE_LEFT, LINE_LEFT + LINE_WIDTH, top, top + LINE_HEIGHT), Claim::LINE_LIMIT);
			lines[i].set_enter_callback([this, i](std::string) { focus((i + 1) % lines.size()); });
			lines[i].set_key_callback(KeyAction::Id::DOWN, [this, i]() { focus((i + 1) % lines.size()); });
			lines[i].set_key_callback(KeyAction::Id::UP, [this, i]() { focus((i + lines.size() - 1) % lines.size()); });
			lines[i].set_key_callback(KeyAction::Id::ESCAPE, [this]() { close(); });
		}

		lines[0].set_state(Textfield::State::FOCUSED);
	}

	void UIClaim::draw(float inter) const
	{
		backgrnd.draw(position);

		for (const auto &line : lines)
			line.draw(position);

		UIElement::draw(inter);
	}

	void UIClaim::update()
	{
		UIElement::update();

		for (auto &line : lines)
			line.update(position);
	}

	Cursor::State UIClaim::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// An open list covers what is under it.
		for (uint16_t id : { Buttons::CHARACTER, Buttons::REASON })
		{
			auto &combo = buttons[id];

			if (combo->is_pressed() && combo->in_combobox(cursorpos))
			{
				Cursor::State state = combo->send_cursor(clicked, cursorpos);

				return clicked && state == Cursor::State::IDLE ? Cursor::State::CLICKING : state;
			}
		}

		for (auto &line : lines)
			if (Cursor::State state = line.send_cursor(cursorpos, clicked); state != Cursor::State::IDLE)
				return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIClaim::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			close();
	}

	UIElement::Type UIClaim::get_type() const
	{
		return TYPE;
	}

	Button::State UIClaim::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::CHARACTER:
			case Buttons::REASON:
				buttons[buttonid]->toggle_pressed();
				break;
			case Buttons::OK:
				confirm();
				break;
			default:
				close();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIClaim::confirm()
	{
		uint16_t chosen = buttons[Buttons::CHARACTER]->get_selected();

		if (chosen < 1 || chosen > names.size())
		{
			warn("Please select the character you'd like to report.");

			return;
		}

		std::vector<std::string> texts;

		for (const auto &line : lines)
			texts.push_back(line.get_text());

		std::string description = Claim::description(texts);

		if (description.empty())
		{
			warn("Please enter the reason for reporting.");

			return;
		}

		const std::string &victim = names[chosen - 1];
		uint8_t reason = Claim::reason_code(kind, buttons[Buttons::REASON]->get_selected());
		std::string chat_log;

		// v83 stars out the reporter's own curse words (0x490b94).
		if (kind == Claim::Kind::CONVERSATION)
			chat_log = Claim::chat_log(rows, victim, Stage::get().get_player().get_stats().get_name(),
					[](const std::string &line) { return CurseData::filter(line); });

		ClaimPacket(kind == Claim::Kind::CONVERSATION, victim, reason, description, chat_log).dispatch();

		close();
	}

	void UIClaim::warn(const std::string &message)
	{
		// A focused line would take the box's keys, so none is until it
		// closes.
		for (auto &line : lines)
			line.set_state(Textfield::State::NORMAL);

		tell(message, [this]() {
			if (is_active())
				focus(0);
		});
	}

	void UIClaim::close()
	{
		for (auto &line : lines)
			line.set_state(Textfield::State::DISABLED);

		deactivate();
	}

	void UIClaim::focus(size_t line)
	{
		for (size_t i = 0; i < lines.size(); i++)
			lines[i].set_state(i == line ? Textfield::State::FOCUSED : Textfield::State::NORMAL);
	}
}
