#include "../IO/GmCommand.h"

#include <iostream>
#include <stdexcept>

namespace
{
	void require(bool condition, const char* message)
	{
		if (!condition)
			throw std::runtime_error(message);
	}

	using ms::GmCommand;
	using Kind = GmCommand::Kind;
}

int main()
{
	try
	{
		// A GM account (grade 0x01) is rank 1, a player's 5.
		GmCommand hide = GmCommand::parse("/h 1", 4);
		require(hide.kind == Kind::HIDE && hide.on, "/h 1 hides");
		require(!GmCommand::parse("/H 0", 1).on && GmCommand::parse("/H 0", 1).kind == Kind::HIDE, "/h 0 shows, in any case");
		require(GmCommand::parse("/h", 1).kind == Kind::NONE, "/h needs its number");
		require(GmCommand::parse("/h 1", 5).kind == Kind::NONE, "a player can't hide");

		GmCommand channel = GmCommand::parse("/ch 3", 3);
		require(channel.kind == Kind::CHANNEL && channel.channel == 2, "/ch counts channels from 1");
		require(GmCommand::parse("/ch 3", 4).kind == Kind::NONE, "rank 4 only hides");
		require(GmCommand::parse("/u", 1).kind == Kind::USERS, "/u lists the map's users");
		require(GmCommand::parse("/uclip", 1).kind == Kind::UCLIP, "/uclip");

		GmCommand warn = GmCommand::parse("/w Bob stop that now", 1);
		require(warn.kind == Kind::WARN && warn.name == "Bob" && warn.text == "stop that now", "a warning");
		require(GmCommand::parse("/w Bob", 1).kind == Kind::NONE, "a warning needs its text");

		GmCommand ban = GmCommand::parse("/ban Bob", 1);
		require(ban.kind == Kind::BAN && ban.name == "Bob", "a ban");

		GmCommand notice = GmCommand::parse("/noticeall hello there", 2);
		require(notice.kind == Kind::BROADCAST && notice.scope == GmCommand::WORLD
			&& notice.type == GmCommand::NOTICE && notice.text == "hello there", "a notice to the world");
		GmCommand alert = GmCommand::parse("/alertm hi", 1);
		require(alert.scope == GmCommand::MAP && alert.type == GmCommand::ALERT, "/alertm is /alertmap");
		GmCommand slide = GmCommand::parse("/slidech hi", 1);
		require(slide.scope == GmCommand::CHANNEL_SCOPE && slide.type == GmCommand::SLIDE, "a slide to the channel");
		GmCommand off = GmCommand::parse("/slidemapx", 1);
		require(off.kind == Kind::BROADCAST && off.scope == GmCommand::MAP && off.text.empty(), "/slidemapx sends no text");
		require(GmCommand::parse("/slidemapx hi", 1).kind == Kind::NONE, "and takes none");
		require(GmCommand::parse("/noticeall", 1).kind == Kind::NONE, "a notice needs its text");
		require(GmCommand::parse("/noticeall hi", 3).kind == Kind::NONE, "rank 3 can't broadcast");
		require(GmCommand::parse("/slideax", 2).kind == Kind::SLIDE_AX, "/slideax");
		require(GmCommand::parse("/party", 0).kind == Kind::NONE, "players' commands aren't GM ones");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}

	std::cout << "gm_command tests passed\n";
	return 0;
}
