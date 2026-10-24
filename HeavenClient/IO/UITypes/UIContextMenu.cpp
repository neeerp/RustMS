#include "UIContextMenu.h"

#include "UIChatBar.h"
#include "UICharInfo.h"
#include "UIClaim.h"
#include "UITradingRoom.h"

#include "../FamilyActions.h"
#include "../GuildCommands.h"
#include "../PartyCommands.h"
#include "../UI.h"
#include "../Window.h"

#include "../Components/ContextMenuLayout.h"
#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Net/Packets/PlayerInteractionPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The rows' art, by context_menu::Entry.
		const char* const BUTTONS[] = {
			"BtInfo", "BtSave", "BtTrade", "", "BtWhisper", "BtParty", "BtGuild", "BtClaim", "BtFamily"
		};

		// The rows' buttons, against the menu's top-left corner.
		constexpr int16_t BUTTON_X = 9, BUTTON_Y = 24;
		// The middle strips and the bottom cap.
		constexpr int16_t STRIP_X = 3;
		// The target's name in the top cap, cut to fit. v83 puts the text's
		// top at (11, 6); HC's glyphs sit about 9 lower than where it draws.
		const Point<int16_t> TITLE_AT(11, -3);
		constexpr size_t TITLE_WIDTH = 78;

	}

	UIContextMenu::UIContextMenu(int32_t c, const std::string& n, Point<int16_t> cursor) :
		UIElement(Point<int16_t>(), Point<int16_t>()), cid(c), name(n)
	{
		nl::node menu = NxFiles::ui_classic["UIWindow.img"]["ContextMenu"];

		top = menu["t"];
		middle = menu["c"];
		bottom = menu["s"];

		// Only the party's leader, or someone in none, sees the party invite.
		Player& player = Stage::get().get_player();
		const Party& party = player.get_party();
		bool party_invite = !party.exists() || party.is_leader(player.get_oid());

		std::vector<context_menu::Entry> entries =
			context_menu::entries(party_invite, GuildCommands::can_invite(player), FamilyActions::can_add_junior());
		rows = entries.size();

		for (size_t row = 0; row < rows; row++)
		{
			Point<int16_t> at(BUTTON_X, BUTTON_Y + static_cast<int16_t>(row) * context_menu::ROW);
			buttons[entries[row]] = std::make_unique<MapleButton>(menu[BUTTONS[entries[row]]], at);
		}

		title = Text(Text::A12M, Text::LEFT, Color::Name::WHITE, name);
		string_format::format_with_ellipsis(title, TITLE_WIDTH);

		auto& constants = Constants::Constants::get();
		position = context_menu::place(cursor, rows, constants.get_viewwidth(), constants.get_viewheight());
		dimension = Point<int16_t>(context_menu::WIDTH, context_menu::height(rows));
	}

	void UIContextMenu::draw(float alpha) const
	{
		top.draw(position);

		for (size_t row = 0; row < rows; row++)
			middle.draw(position + Point<int16_t>(STRIP_X, context_menu::TOP + static_cast<int16_t>(row) * context_menu::ROW));

		bottom.draw(position + Point<int16_t>(STRIP_X, context_menu::TOP + static_cast<int16_t>(rows) * context_menu::ROW));
		title.draw(position + TITLE_AT);

		UIElement::draw_buttons(alpha);
	}

	// A right button coming up outside counts as a press there (FUN_00997379).
	void UIContextMenu::rightclick(Point<int16_t> cursorpos)
	{
		if (!is_in_range(cursorpos))
			deactivate();
	}

	// A press outside closes the menu (FUN_0099735f).
	Cursor::State UIContextMenu::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (clicked && !is_in_range(cursorpos))
		{
			deactivate();

			return Cursor::State::IDLE;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	// ESC closes the menu, and so does Enter, picking nothing.
	void UIContextMenu::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			deactivate();
	}

	UIElement::Type UIContextMenu::get_type() const
	{
		return TYPE;
	}

	// The menu closes first, then acts (FUN_0095009a).
	Button::State UIContextMenu::button_pressed(uint16_t buttonid)
	{
		deactivate();

		Player& player = Stage::get().get_player();
		auto chatbar = UI::get().get_element<UIChatBar>();

		switch (buttonid)
		{
			case context_menu::INFO:
				// As a double click on them (FUN_00a23fba).
				if (Stage::get().get_character(cid) && player.request(CharInfoRequestPacket(cid), ExclRequest::USE_WAIT))
					UI::get().emplace<UICharInfo>(cid);
				break;
			case context_menu::SAVE_NAME:
				// Copied for pasting into the chat, which opens (FUN_009f967a).
				Window::get().setclipboard(name);

				if (chatbar)
					chatbar->toggle_chatfield(true);
				break;
			case context_menu::TRADE:
				UITradingRoom::request(cid);
				break;
			case context_menu::WHISPER:
				if (chatbar)
					chatbar->whisper_to(name);
				break;
			case context_menu::PARTY:
				PartyCommands::invite(name);
				break;
			case context_menu::GUILD:
				GuildCommands::invite(name);
				break;
			case context_menu::FAMILY:
				FamilyActions::add_junior(name);
				break;
			case context_menu::CLAIM:
				UIClaimNotice::open(name);
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}
}
