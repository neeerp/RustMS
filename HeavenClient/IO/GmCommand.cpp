#include "GmCommand.h"

#include "ChatCommand.h"

#include <cstdlib>
#include <vector>

namespace ms
{
	namespace
	{
		struct Broadcast
		{
			const char* command;
			bool argument;
			uint8_t scope;
			uint8_t type;
		};

		// In v83's order (0x52cf45..0x52d39c); an `x` sends no text, which
		// takes the slide down.
		constexpr Broadcast BROADCASTS[] = {
			// StringPool 0x6e9, 0x6eb, 0x104, 0x6e7
			{ "/alertch", true, GmCommand::CHANNEL_SCOPE, GmCommand::ALERT },
			{ "/alertmap", true, GmCommand::MAP, GmCommand::ALERT },
			{ "/alertm", true, GmCommand::MAP, GmCommand::ALERT },
			{ "/alertall", true, GmCommand::WORLD, GmCommand::ALERT },
			// StringPool 0x6e8, 0x6ea, 0x6ec, 0x105
			{ "/noticeall", true, GmCommand::WORLD, GmCommand::NOTICE },
			{ "/noticech", true, GmCommand::CHANNEL_SCOPE, GmCommand::NOTICE },
			{ "/noticemap", true, GmCommand::MAP, GmCommand::NOTICE },
			{ "/noticem", true, GmCommand::MAP, GmCommand::NOTICE },
			// StringPool 0xaff, 0xb06, 0xb00, 0xb07, 0xb01, 0xb08
			{ "/slideall", true, GmCommand::WORLD, GmCommand::SLIDE },
			{ "/slideallx", false, GmCommand::WORLD, GmCommand::SLIDE },
			{ "/slidech", true, GmCommand::CHANNEL_SCOPE, GmCommand::SLIDE },
			{ "/slidechx", false, GmCommand::CHANNEL_SCOPE, GmCommand::SLIDE },
			{ "/slidemap", true, GmCommand::MAP, GmCommand::SLIDE },
			{ "/slidemapx", false, GmCommand::MAP, GmCommand::SLIDE }
		};

		GmCommand of(GmCommand::Kind kind)
		{
			GmCommand command;
			command.kind = kind;
			return command;
		}
	}

	GmCommand GmCommand::parse(const std::string& input, int rank)
	{
		std::string rest;

		// StringPool 0x6e6
		if (rank <= 4 && ChatCommand::with_argument(input, "/h", rest))
		{
			GmCommand command = of(Kind::HIDE);
			command.on = std::atoi(rest.c_str()) != 0;
			return command;
		}

		if (rank <= 3)
		{
			// StringPool 0x6e4, 0xbf
			if (ChatCommand::with_argument(input, "/ch", rest))
			{
				GmCommand command = of(Kind::CHANNEL);
				command.channel = std::atoi(rest.c_str()) - 1;
				return command;
			}

			if (ChatCommand::same_text(input, "/u"))
				return of(Kind::USERS);

			// StringPool 0x700: a name, a space, and the rest (FUN_004a8f57).
			if (ChatCommand::with_argument(input, "/w", rest))
			{
				size_t space = rest.find(' ');

				if (space != std::string::npos && space > 0 && space + 1 < rest.size())
				{
					GmCommand command = of(Kind::WARN);
					command.name = rest.substr(0, space);
					command.text = rest.substr(space + 1);
					return command;
				}
			}

			// StringPool 0x6e5
			if (ChatCommand::with_argument(input, "/ban", rest))
			{
				GmCommand command = of(Kind::BAN);
				command.name = rest;
				return command;
			}

			// StringPool 0x6e1: exactly three words after it, the command
			// itself matched as typed (0x4a91b0).
			if (input.compare(0, 7, "/block ") == 0)
			{
				std::vector<std::string> words;
				size_t start = 7;

				for (size_t space; (space = input.find(' ', start)) != std::string::npos; start = space + 1)
					words.push_back(input.substr(start, space - start));

				words.push_back(input.substr(start));

				if (words.size() == 3)
				{
					GmCommand command = of(Kind::BLOCK);
					command.name = words[0];
					command.days = std::atoi(words[1].c_str());
					command.text = words[2];
					return command;
				}
			}

			// StringPool 0x6df
			if (ChatCommand::same_text(input, "/uclip"))
				return of(Kind::UCLIP);
		}

		if (rank <= 2)
		{
			for (const Broadcast& broadcast : BROADCASTS)
			{
				bool matched = broadcast.argument
					? ChatCommand::with_argument(input, broadcast.command, rest)
					: ChatCommand::same_text(input, broadcast.command);

				if (!matched)
					continue;

				GmCommand command = of(Kind::BROADCAST);
				command.scope = broadcast.scope;
				command.type = broadcast.type;
				command.text = broadcast.argument ? rest : "";
				return command;
			}

			// StringPool 0xb09
			if (ChatCommand::same_text(input, "/slideax"))
				return of(Kind::SLIDE_AX);
		}

		return GmCommand();
	}

	int32_t GmCommand::block_reason(const std::vector<std::string>& types) const
	{
		for (size_t i = 0; i < types.size(); i++)
			if (types[i] == text)
				return static_cast<int32_t>(i + 1);

		return 0;
	}
}
