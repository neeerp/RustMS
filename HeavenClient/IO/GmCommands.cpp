#include "GmCommands.h"

#include "GmCommand.h"

#include "UI.h"

#include "UITypes/UIGmBlock.h"

#include "../Configuration.h"
#include "../Gameplay/Stage.h"
#include "../Net/Packets/GameplayPackets.h"

namespace ms
{
	namespace GmCommands
	{
		bool run(const std::string& input)
		{
			const AccountGrade& grade = Configuration::get().get_grade();

			// v83 reads the GM commands only for these grades (0x52c9cd).
			if (!grade.slash_commands())
				return false;

			GmCommand command = GmCommand::parse(input, grade.rank());

			switch (command.kind)
			{
				case GmCommand::Kind::NONE:
					return false;
				case GmCommand::Kind::HIDE:
					AdminHidePacket(command.on).dispatch();
					break;
				case GmCommand::Kind::CHANNEL:
					// FUN_005304af, as the channel menu asks, with no check of
					// the number.
					Stage::get().change_channel(static_cast<uint8_t>(command.channel));
					break;
				case GmCommand::Kind::USERS:
					AdminEnterMapPacket(AdminEnterMapPacket::Operation::SHOW_USERS).dispatch();
					break;
				case GmCommand::Kind::WARN:
					AdminWarnPacket(command.name, command.text).dispatch();
					break;
				case GmCommand::Kind::BAN:
					AdminBanPacket(command.name).dispatch();
					break;
				case GmCommand::Kind::BLOCK:
				{
					// With no days or no such reason, only the log goes
					// (0x52dc30). v83's dialog is modal, so its 0x04 goes
					// before the log; here it goes when OK is pressed.
					std::vector<std::string> reasons = UIGmBlock::reasons();
					int32_t reason = command.block_reason(reasons);

					if (command.days != 0 && reason != 0)
						UI::get().emplace<UIGmBlock>(command.name, command.days, reason, reasons);

					break;
				}
				case GmCommand::Kind::UCLIP:
					AdminEnterMapPacket(AdminEnterMapPacket::Operation::ALERT_ADMINS).dispatch();
					break;
				case GmCommand::Kind::BROADCAST:
					AdminBroadcastPacket(command.scope, command.type, command.text).dispatch();
					break;
				case GmCommand::Kind::SLIDE_AX:
					GmSlidePacket().dispatch();
					break;
			}

			// Every GM command is logged, as typed (0x52e28f).
			AdminLogPacket(input).dispatch();
			return true;
		}
	}
}
