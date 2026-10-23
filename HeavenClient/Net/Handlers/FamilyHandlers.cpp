#include "FamilyHandlers.h"

#include "Helpers/FamilyParser.h"

#include "../Packets/FamilyPackets.h"

#include "../../Character/Blacklist.h"
#include "../../Configuration.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/FamilyActions.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIBuffList.h"
#include "../../IO/UITypes/UIFamily.h"
#include "../../IO/UITypes/UIFamilyTree.h"
#include "../../IO/UITypes/UIInviteCard.h"
#include "../../IO/UITypes/UINotice.h"

#include <cstdlib>

namespace ms
{
	namespace
	{
		// FAMILY_RESULT's codes (0xa345fb, report b039 1.3) with their
		// StringPool texts. 0x4b and 0x4c go to the chat log; the others
		// are OK dialogs. Code 1 (0x120a) names whom the player parted
		// from, by the name kept when they said yes (report b048).
		const char* result_text(int32_t code)
		{
			switch (code)
			{
			case 1: return "You have severed ties with %s.\\nYour family relationship has ended.";
			case 0x40: return "You cannot add this character as a Junior.";
			case 0x41: return "The name you entered is incorrect or he/she is currently not logged in.";
			case 0x42: return "You belong to the same family.";
			case 0x43: return "You do not belong to the same family.";
			case 0x45: return "The character you wish to add as\\na Junior must be in the same map.";
			case 0x46: return "This character is already a Junior of another character.";
			case 0x47: return "The Junior you wish to add\\nmust be at a lower rank.";
			case 0x48: return "The gap between you and your\\njunior must be within 20 levels.";
			case 0x49: return "Another character has requested to add this character.\\nPlease try again later.";
			case 0x4a: return "Another character has requested a summon.\\nPlease try again later.";
			case 0x4b: return "The summons has failed. Your current location or state does not allow a summons.";
			case 0x4c: return "The family cannot extend more than 1000 generations from above and below.";
			case 0x4d: return "The Junior you wish to add\\nmust be over Level 10.";
			case 0x4e: return "You cannot add a Junior that has requested to change worlds.";
			case 0x4f: return "You cannot add a Junior since you've requested to change worlds.";
			case 0x50: return "Separation is not possible due to insufficient Mesos. You will need %d Mesos to separate with a Senior.";
			case 0x51: return "Separation is not possible due to insufficient Mesos. You will need %d Mesos to separate with a Junior.";
			case 0x52: return "The Entitlement does not apply because your level does not match the corresponding area.";
			default: return nullptr;
			}
		}

		std::string with_name(std::string text, const std::string& name)
		{
			size_t at = text.find("%s");

			if (at != std::string::npos)
				text.replace(at, 2, name);

			return text;
		}

		std::string with_number(std::string text, int32_t value)
		{
			size_t at = text.find("%d");

			if (at != std::string::npos)
				text.replace(at, 2, std::to_string(value));

			return text;
		}

		void ok(const std::string& text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}

		void refresh_window()
		{
			if (auto window = UI::get().get_element<UIFamily>())
				window->refresh();
		}

		// The tree, when it is open and the player's own.
		Optional<UIFamilyTree> own_tree()
		{
			auto tree = UI::get().get_element<UIFamilyTree>();

			if (tree && tree->is_active() && tree->is_mine())
				return tree;

			return {};
		}
	}

	void FamilyChartHandler::handle(InPacket& recv) const
	{
		auto tree = UI::get().get_element<UIFamilyTree>();

		if (!tree || !tree->is_active())
			return;

		const Player& player = Stage::get().get_player();

		FamilyNode self;
		self.id = player.get_oid();
		self.job = player.get_stats().get_job().get_id();
		self.level = static_cast<uint8_t>(player.get_level());
		self.name = player.get_stats().get_name();

		FamilyChart chart = FamilyParser::parse_chart(recv, self.id);
		chart.arrange(self);

		tree->set_chart(std::move(chart));
	}

	void FamilyInfoHandler::handle(InPacket& recv) const
	{
		Player& player = Stage::get().get_player();
		player.get_family().set_info(FamilyParser::parse_info(recv));

		// The own tree asks again for what changed (0xa344f8).
		if (own_tree())
			OpenFamilyPedigreePacket(player.get_stats().get_name()).dispatch();

		refresh_window();
	}

	void FamilyResultHandler::handle(InPacket& recv) const
	{
		int32_t code = recv.read_int();
		int32_t arg = recv.read_int();

		const char* text = result_text(code);

		if (!text)
			return;

		if (code == 0x4b || code == 0x4c)
			FamilyActions::chat_line(text);
		else if (code == 1)
			ok(with_name(text, FamilyActions::parted_from()));
		else
			ok(with_number(text, arg));
	}

	// A card, unless one is up already, the Game Option blocks it or the
	// inviter is on the Blacklist, when the client says no itself (reports
	// b039 1.4, b091). Its X says no; letting it fade
	// says nothing. O asks once more before saying yes.
	void FamilyJoinRequestHandler::handle(InPacket& recv) const
	{
		int32_t inviter = recv.read_int();
		std::string name = recv.read_string();

		auto cards = UI::get().get_element<UIInviteCard>();

		if (!cards || cards->showing(UIInviteCard::FAMILY) || !Setting<AllowFamilyInvite>::get().load() || Blacklist::listed(name))
			return AcceptFamilyPacket(inviter, name, false).dispatch();

		cards->add(UIInviteCard::FAMILY, name, [inviter, name](bool yes)
		{
			if (!yes)
				return AcceptFamilyPacket(inviter, name, false).dispatch();

			// StringPool 0x1205.
			std::string question = name + " would like to\\nbe your Senior. Would you like to be\\n" + name + "'s Junior?";

			UI::get().emplace<UIYesNo>(question, [inviter, name](bool accept)
			{
				AcceptFamilyPacket(inviter, name, accept).dispatch();
			});
		});
	}

	void FamilyJoinRequestResultHandler::handle(InPacket& recv) const
	{
		bool accepted = recv.read_bool();
		std::string junior = recv.read_string();

		// StringPool 0x1207 and 0x1206.
		if (accepted)
			FamilyActions::chat_line("You have added " + junior + " as a Junior. Please do your best to guide the Junior in his/her game play.");
		else
			ok(junior + " has declined your Junior request.");
	}

	void FamilyJoinAcceptedHandler::handle(InPacket& recv) const
	{
		std::string senior = recv.read_string();

		// StringPool 0x1208.
		FamilyActions::chat_line(senior + " has become your Senior. He/she will guide you in your game play.");
	}

	void FamilyPrivilegeListHandler::handle(InPacket& recv) const
	{
		Stage::get().get_player().get_family().set_privileges(FamilyParser::parse_privileges(recv));

		refresh_window();
	}

	// The numbers in the window wait for the INFO that follows.
	void FamilyRepGainHandler::handle(InPacket& recv) const
	{
		int32_t amount = recv.read_int();
		std::string name = recv.read_string();

		// StringPool 0x120b and 0x120c.
		if (amount >= 0)
			FamilyActions::chat_line("Your Rep has increased (" + name + ", +" + std::to_string(amount) + ")");
		else
			FamilyActions::chat_line("You have used " + std::to_string(std::abs(amount)) + " Rep Points.");
	}

	void FamilyNotifyLoginOrLogoutHandler::handle(InPacket& recv) const
	{
		bool online = recv.read_bool();
		std::string name = recv.read_string();

		if (online)
			if (auto cards = UI::get().get_element<UIInviteCard>())
				cards->notify(name, false);

		if (auto tree = own_tree())
			tree->set_online(name, online);
	}

	// A new one takes the old one's icon away (0xa34fa4). The client never
	// takes it away by itself: the server sends 0 when it ends.
	void FamilySetPrivilegeHandler::handle(InPacket& recv) const
	{
		FamilyPrivilegeInUse in_use = FamilyParser::parse_in_use(recv);

		Stage::get().get_player().get_family().set_in_use(in_use.index);

		if (auto bufflist = UI::get().get_element<UIBuffList>())
			bufflist->set_family(in_use.icon, in_use.left);
	}

	// A modal Yes/No with nothing gating it; any answer but Yes is a no
	// (report b050 3).
	void FamilySummonRequestHandler::handle(InPacket& recv) const
	{
		std::string summoner = recv.read_string();
		std::string map = recv.read_string();

		// StringPool 0x1210.
		std::string question = "From " + map + ", " + summoner + " has requested a summons. Would you like to transport?";

		UI::get().emplace<UIYesNo>(question, [summoner](bool yes)
		{
			FamilySummonResponsePacket(summoner, yes).dispatch();
		});
	}
}
