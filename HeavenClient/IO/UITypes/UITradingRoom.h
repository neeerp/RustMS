#pragma once

#include "../UIDragElement.h"

#include "../Components/Slider.h"
#include "../Components/Textfield.h"

#include "../../Character/Look/CharLook.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"
#include "../../Net/Handlers/MiniRoom.h"

#include <array>
#include <vector>

namespace ms
{
	// The trading room (v83 CTradingRoomDlg): the partner's offer on the
	// left, ours on the right, and a chat between them. The server opens it
	// once we create or join a trade; it closes when the trade ends.
	class UITradingRoom : public UIDragElement<PosTRADE>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::TRADINGROOM;
		// Not focused, so items can be dragged in from the inventory.
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UITradingRoom(const mini_room::Enter &enter);

		void draw(float inter) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// Someone took the other seat.
		void visit(const mini_room::Member &member);

		// A member's equipment changed.
		void change_look(uint8_t slot, const LookEntry &look);

		// A line the member in `slot` said, "name : text"; the partner's
		// words are filtered.
		void add_chat(uint8_t slot, const std::string &line);

		void put_item(const mini_room::PutItem &put);

		void put_meso(const mini_room::PutMeso &put);

		// The partner pressed Trade.
		void partner_confirmed();

		// Our mesos went over the daily limit; Trade can be pressed again.
		void meso_limit();

		// The trade ended; closes the window, saying why if there is a reason.
		void leave(int8_t reason);

		// Closes the window without telling the server, which ends the trade
		// itself when we leave the field.
		void close();

		// Asks cid to trade, as the character info window and the menu on
		// someone do (FUN_0052fad4): one room at a time, with someone else
		// on the map.
		static void request(int32_t cid);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void cancel();

		void ask_meso();

		void ask_trade();

		void confirm();

		void submit(const std::string &text);

		void add_line(const std::string &line, Color::Name color);

		void scroll_to_bottom();

		void show_item(Point<int16_t> cursor);

		void clear_tooltip();

		void refresh_buttons();

		bool can_offer() const;

		enum Buttons : uint16_t
		{
			TRADE,
			COIN,
			RESET,
			ENTER,
			CLAIM
		};

		struct Member
		{
			bool present = false;
			CharLook look;
			Text name;
		};

		Texture backgrnd;

		std::array<Member, 2> members;
		uint8_t own_slot;

		std::array<mini_room::Offer, 2> offers;
		std::array<Text, 2> meso_labels;
		// For the mesos received after fees, as v83 counts them.
		int64_t meso_at_open;

		struct Line
		{
			Text text;
			int16_t rows;
		};

		std::vector<Line> log;
		int16_t log_rows;
		int16_t log_row;
		Slider slider;
		Textfield input;
	};
}
