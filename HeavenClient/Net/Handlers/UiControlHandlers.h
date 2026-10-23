#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// Opens one of the player's windows if it isn't open (v83 FUN_00a05ab8,
	// report b097).
	// Opcode: OPEN_UI(220)
	class OpenUiHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Takes the HUD, the windows and the keys away, or gives them back
	// (FUN_0095ff5a).
	// Opcode: LOCK_UI(221)
	class LockUiHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Hides other players' comings and doings (ctx+0x30b4).
	// Opcode: DISABLE_UI(222)
	class DisableUiHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// A line in the chat log (0x961234).
	// Opcode: MESSAGE_BOX(232)
	class MessageBoxHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// A slideshow of images by path (0x961275).
	// Opcode: OPEN_UI_STRING_LIST(233)
	class OpenUiStringListHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
