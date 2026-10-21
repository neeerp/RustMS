#include "BuddyCommands.h"

#include "UI.h"

#include "UITypes/UIInviteCard.h"
#include "UITypes/UINotice.h"

#include "../Configuration.h"

#include "../Gameplay/Stage.h"
#include "../Net/Packets/GameplayPackets.h"
#include "../Util/CharacterName.h"

#include <algorithm>
#include <cctype>

namespace ms
{
	namespace BuddyCommands
	{
		namespace
		{
			void notice(const std::string& message)
			{
				UI::get().emplace<UIOk>(message, [](bool) {});
			}

			bool same_name(const std::string& a, const std::string& b)
			{
				return a.size() == b.size() && std::equal(a.begin(), a.end(), b.begin(),
					[](unsigned char x, unsigned char y) { return std::tolower(x) == std::tolower(y); });
			}
		}

		void add(const std::string& name, const std::string& group, bool notify)
		{
			const Player& player = Stage::get().get_player();
			const Buddy* listed = player.get_buddies().find_listed(name);

			auto refuse = [notify](const std::string& text)
			{
				if (notify)
					notice(text);
			};

			if (!character_name_valid(name))
				refuse("You have entered an invalid character name.");
			else if (listed && listed->group == group)
				refuse("The buddy is already registered.");
			else if (same_name(name, player.get_stats().get_name()))
				refuse("You can't enter yourself as your buddy.");
			else
				AddBuddyPacket(name, group).dispatch();
		}

		void accept(int32_t cid)
		{
			const Buddy* buddy = Stage::get().get_player().get_buddies().find(cid);

			if (!buddy || buddy->pending())
				AcceptBuddyPacket(cid).dispatch();
			else
				notice("You have not requested buddies with that user.");
		}

		void remove(int32_t cid)
		{
			if (Stage::get().get_player().get_buddies().find(cid))
				DeleteBuddyPacket(cid).dispatch();
			else
				notice("The user is not registered.");
		}

		void ask(const std::string& name, int32_t cid)
		{
			if (auto cards = UI::get().get_element<UIInviteCard>())
				cards->add(UIInviteCard::BUDDY, name, [cid](bool yes)
				{
					if (yes)
						accept(cid);
					else
						remove(cid);
				});
		}

		void show_requests()
		{
			auto cards = UI::get().get_element<UIInviteCard>();

			if (!cards || cards->showing(UIInviteCard::BUDDY))
				return;

			for (const Buddy& buddy : Stage::get().get_player().get_buddies().get_buddies())
				if (buddy.pending())
					ask(buddy.name, buddy.id);
		}

		FriendGroups::Blocks blocks()
		{
			return FriendGroups::Blocks(FriendGroups::parse_blocks(Setting<FriendBlocked>::get().load(),
					Stage::get().get_player().get_oid()));
		}
	}
}
