#pragma once

#include "../../Character/Look/CharLook.h"
#include "../../Graphics/Animation.h"
#include "../../Graphics/Text.h"
#include "../../Net/Login.h"
#include "../../Template/Point.h"
#include "../../Util/MapleTv.h"

#include <nlnx/node.hpp>

#include <array>
#include <cstdint>
#include <optional>
#include <string>

namespace ms
{
	// The screens of a Maple TV NPC (Npc.nx info MapleTV, e.g. Orbis's
	// 9250046). v83's NPC template loader FUN_006dce02 reads MapleTV and the
	// anchors MapleTVmsgX/Y and MapleTVadX/Y (+0x48 to +0x58), which are
	// offsets from the NPC's foot, never flipped; the NPC's update
	// FUN_006d2918 runs the TV while MapleTV is set. Its layers are above
	// the NPC's (report b-p41-maple-tv-npc). Idle, an ad loops below and the
	// body is off above; a message (recv 341) takes the body's place until
	// recv 342 turns it off again (report b-p42-maple-tv-message).
	class MapleTvScreen
	{
	public:
		// What recv 341 put on every TV (the manager DAT_00bed76c): its
		// type (0 plain, 1 star, 2 heart), the sender's look and the
		// receiver's if sent, the names and the five lines.
		struct Message
		{
			uint8_t type;
			LookEntry sender_look;
			std::optional<LookEntry> receiver_look;
			std::string sender;
			std::string receiver;
			MapleTv::Lines lines;
		};

		// Every TV shows the message, the TVs of maps entered later too,
		// until the next one or clear().
		static void show(Message message);
		static void clear();

		// The anchors as the NPC's info gives them: the NX files store the
		// negative msgX of most TVs as an unsigned short (65516 for -20).
		struct Anchors
		{
			Point<int16_t> message;
			Point<int16_t> ad;
		};

		static bool enabled(nl::node info);
		static Anchors anchors(nl::node info);

		// Where the art goes from the NPC's foot: the body (240x90, origin
		// (0,89)) at (msgX, msgY + 0x5a), the ad (240x180, origin (0,180)) at
		// (adX, adY + 0xb4), report c056 1.2.
		static Point<int16_t> body_position(Anchors anchors);
		static Point<int16_t> ad_position(Anchors anchors);

		MapleTvScreen();
		MapleTvScreen(nl::node info, int32_t ad);

		void update();
		void draw(Point<int16_t> foot, float alpha) const;

	private:
		void show_message(const Message& message);
		void turn_off();

		bool on;
		Anchors at;
		Animation ad;
		Animation body;

		// The last message (or clearing) this TV has taken up.
		uint32_t seen;
		bool showing;
		uint8_t type;
		Animation frame;
		std::optional<CharLook> sender_look;
		std::optional<CharLook> receiver_look;
		Text sender;
		Text receiver;
		int16_t receiver_x;
		std::array<Text, MapleTv::ROWS> lines;
	};
}
